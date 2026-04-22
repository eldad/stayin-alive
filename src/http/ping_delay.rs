use axum::{extract::Query, http::StatusCode, response::IntoResponse};
use rand::Rng as _;
use serde::Deserialize;
use std::time::Duration;
use tokio::time::sleep;

/// Maximum total wait time (delay + jitter) in seconds.
const MAX_TOTAL_SECONDS: f64 = 120.0;

/// Query parameters accepted by the `/ping-delay` endpoint.
#[derive(Debug, Deserialize)]
pub struct PingDelayParams {
    /// Base delay in seconds before replying. Defaults to 5.
    #[serde(default = "default_delay")]
    delay: f64,
    /// Maximum random jitter (±jitter seconds) added to the delay. Defaults to 1.
    #[serde(default = "default_jitter")]
    jitter: f64,
}

fn default_delay() -> f64 {
    5.0
}

fn default_jitter() -> f64 {
    1.0
}

/// GET /ping-delay — waits `delay` seconds ± random `jitter`, then replies "ping".
///
/// Both `delay` and `jitter` are optional query parameters.  The combined
/// maximum (`delay + jitter`) must not exceed 120 seconds; requests that
/// exceed this limit are rejected with 400 Bad Request.
pub async fn ping_delay_handler(Query(params): Query<PingDelayParams>) -> impl IntoResponse {
    if params.delay < 0.0 || params.jitter < 0.0 {
        return (
            StatusCode::BAD_REQUEST,
            "delay and jitter must be non-negative",
        )
            .into_response();
    }
    if params.delay + params.jitter > MAX_TOTAL_SECONDS {
        return (
            StatusCode::BAD_REQUEST,
            "delay + jitter must not exceed 120 seconds",
        )
            .into_response();
    }

    let jitter_offset = rand::rng().random_range(-params.jitter..=params.jitter);
    let total = (params.delay + jitter_offset).max(0.0);
    let wait = Duration::from_secs_f64(total);

    sleep(wait).await;

    (StatusCode::OK, "ping").into_response()
}
