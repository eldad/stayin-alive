use std::sync::Arc;

use axum::{
    extract::{MatchedPath, Request},
    middleware::Next,
    response::IntoResponse,
    Extension,
};
use metrics_exporter_prometheus::{Matcher, PrometheusBuilder, PrometheusHandle};
use tokio::time::Instant;

const METRIC_HTTP_REQUEST_DURATION: &str = "http_request_duration_seconds";
const METRIC_HTTP_REQUESTS_TOTAL: &str = "http_requests_total";
const METRIC_ACTIVE_CONNECTIONS: &str = "active_connections";

const HTTP_REQUEST_DURATION_BUCKETS: &[f64] = &[
    0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
];

/// Install the Prometheus metrics recorder and return a handle for rendering.
///
/// Initialises all per-protocol connection gauges to 0 so they appear in the
/// first scrape even before any connections arrive.
pub fn install() -> Result<PrometheusHandle, metrics_exporter_prometheus::BuildError> {
    let handle = PrometheusBuilder::new()
        .set_buckets_for_metric(
            Matcher::Full(METRIC_HTTP_REQUEST_DURATION.into()),
            HTTP_REQUEST_DURATION_BUCKETS,
        )?
        .install_recorder()?;

    for protocol in &["http", "websocket", "sse", "grpc"] {
        metrics::gauge!(METRIC_ACTIVE_CONNECTIONS, "protocol" => *protocol).set(0.0);
    }

    Ok(handle)
}

/// RAII guard that increments a connection gauge on creation and decrements it on drop.
///
/// Use this to track the lifetime of any long-lived protocol connection:
/// WebSocket, SSE, or gRPC RPC.
pub struct ConnectionGuard {
    protocol: &'static str,
}

impl ConnectionGuard {
    pub fn new(protocol: &'static str) -> Self {
        metrics::gauge!(METRIC_ACTIVE_CONNECTIONS, "protocol" => protocol).increment(1.0);
        Self { protocol }
    }
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        metrics::gauge!(METRIC_ACTIVE_CONNECTIONS, "protocol" => self.protocol).decrement(1.0);
    }
}

/// Axum middleware that records per-path request latency (histogram) and
/// request count (counter) for every HTTP request, and additionally tracks
/// in-flight plain HTTP connections via the `active_connections{protocol="http"}`
/// gauge. WebSocket (`/ws`) and SSE (`/sse-ping`) endpoints are excluded from
/// the HTTP connection gauge because they are tracked separately.
pub async fn track_request(req: Request, next: Next) -> impl IntoResponse {
    let path = match req.extensions().get::<MatchedPath>() {
        Some(matched) => matched.as_str().to_owned(),
        None => req.uri().path().to_owned(),
    };
    let method = req.method().as_str().to_owned();

    // WebSocket and SSE connections are counted under their own protocol labels.
    let is_plain_http = path != "/ws" && path != "/sse-ping";
    if is_plain_http {
        metrics::gauge!(METRIC_ACTIVE_CONNECTIONS, "protocol" => "http").increment(1.0);
    }

    let start = Instant::now();
    let response = next.run(req).await;
    let duration = start.elapsed().as_secs_f64();

    if is_plain_http {
        metrics::gauge!(METRIC_ACTIVE_CONNECTIONS, "protocol" => "http").decrement(1.0);
    }

    let status = response.status().as_u16().to_string();
    let labels = [("path", path), ("method", method), ("status", status)];
    metrics::histogram!(METRIC_HTTP_REQUEST_DURATION, &labels).record(duration);
    metrics::counter!(METRIC_HTTP_REQUESTS_TOTAL, &labels).increment(1);

    response
}

/// GET /metrics — render all recorded Prometheus metrics.
pub async fn metrics_handler(Extension(handle): Extension<Arc<PrometheusHandle>>) -> String {
    handle.render()
}
