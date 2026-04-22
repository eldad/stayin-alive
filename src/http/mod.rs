use std::sync::Arc;

use axum::{middleware, routing::get, Extension, Router};
use metrics_exporter_prometheus::PrometheusHandle;
use tower_http::trace::TraceLayer;

mod ping;
mod sse;
mod ws;

pub use ping::ping_handler;
pub use sse::sse_ping_handler;
pub use ws::ws_handler;

/// Build the HTTP router with all keepalive endpoints and Prometheus metrics.
pub fn router(prometheus_handle: Arc<PrometheusHandle>) -> Router {
    Router::new()
        .route("/ping", get(ping_handler))
        .route("/sse-ping", get(sse_ping_handler))
        .route("/ws", get(ws_handler))
        .route("/metrics", get(crate::metrics::metrics_handler))
        .layer(middleware::from_fn(crate::metrics::track_request))
        .layer(TraceLayer::new_for_http())
        .layer(Extension(prometheus_handle))
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, OnceLock};

    use axum::body::Body;
    use http_body_util::BodyExt;
    use metrics_exporter_prometheus::PrometheusHandle;
    use tokio::net::TcpListener;
    use tower::ServiceExt;

    use super::*;

    /// Returns a shared `PrometheusHandle`, installing the recorder exactly once
    /// for the whole test process.
    fn test_handle() -> Arc<PrometheusHandle> {
        static HANDLE: OnceLock<Arc<PrometheusHandle>> = OnceLock::new();
        HANDLE
            .get_or_init(|| Arc::new(crate::metrics::install().expect("install prometheus")))
            .clone()
    }

    /// Helper: build a request for the given URI.
    fn get_request(uri: &str) -> axum::http::Request<Body> {
        axum::http::Request::builder()
            .uri(uri)
            .body(Body::empty())
            .unwrap()
    }

    // ---- /ping ----

    #[tokio::test]
    async fn ping_returns_pong() {
        let app = router(test_handle());
        let resp = app.oneshot(get_request("/ping")).await.unwrap();

        assert_eq!(resp.status(), 200);
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(&body[..], b"pong");
    }

    #[tokio::test]
    async fn ping_wrong_method_returns_405() {
        let app = router(test_handle());
        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/ping")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), 405);
    }

    // ---- /sse-ping ----

    #[tokio::test]
    async fn sse_ping_returns_event_stream() {
        let app = router(test_handle());
        let resp = app
            .oneshot(get_request("/sse-ping?interval_ms=50"))
            .await
            .unwrap();

        assert_eq!(resp.status(), 200);
        let ct = resp
            .headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap();
        assert!(
            ct.contains("text/event-stream"),
            "expected text/event-stream, got {ct}"
        );
    }

    #[tokio::test]
    async fn sse_ping_default_interval() {
        let app = router(test_handle());
        // No interval_ms param — should still succeed with default.
        let resp = app.oneshot(get_request("/sse-ping")).await.unwrap();
        assert_eq!(resp.status(), 200);
    }

    // ---- /ws ----

    #[tokio::test]
    async fn ws_upgrade_requires_websocket_headers() {
        // A plain GET without upgrade headers should be rejected.
        let app = router(test_handle());
        let resp = app.oneshot(get_request("/ws")).await.unwrap();
        assert_eq!(resp.status(), 400);
    }

    #[tokio::test]
    async fn ws_sends_ping_messages() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            axum::serve(listener, router(test_handle())).await.unwrap();
        });

        let url = format!("ws://{addr}/ws?interval_ms=50");
        let (mut ws, _) = tokio_tungstenite::connect_async(&url).await.unwrap();

        use futures::StreamExt;
        // Read 3 messages and verify they are "ping".
        for _ in 0..3 {
            let msg = ws.next().await.unwrap().unwrap();
            assert_eq!(msg.into_text().unwrap(), "ping");
        }
    }

    // ---- /metrics ----

    #[tokio::test]
    async fn metrics_endpoint_returns_prometheus_text() {
        let app = router(test_handle());
        let resp = app.oneshot(get_request("/metrics")).await.unwrap();
        assert_eq!(resp.status(), 200);
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let text = std::str::from_utf8(&body).unwrap();
        assert!(
            text.contains("active_connections"),
            "expected active_connections gauge in metrics output"
        );
    }

    // ---- 404 ----

    #[tokio::test]
    async fn unknown_route_returns_404() {
        let app = router(test_handle());
        let resp = app.oneshot(get_request("/nope")).await.unwrap();
        assert_eq!(resp.status(), 404);
    }
}
