use std::time::Duration;

use axum::{
    extract::Query,
    response::{
        sse::{Event, KeepAlive},
        Sse,
    },
};
use futures::stream::{self, Stream};
use serde::Deserialize;
use tokio_stream::StreamExt as _;

/// Query parameters accepted by the SSE endpoint.
#[derive(Debug, Deserialize)]
pub struct SsePingParams {
    /// Interval between consecutive ping messages, in milliseconds.
    /// Defaults to 1000 ms if not specified.
    #[serde(default = "default_interval_ms")]
    interval_ms: u64,
}

fn default_interval_ms() -> u64 {
    1000
}

/// GET /sse-ping — Server-Sent Events endpoint.
///
/// Keeps the connection open and emits a "ping" event at the requested
/// interval (`interval_ms` query parameter, default 1000 ms).
pub async fn sse_ping_handler(
    Query(params): Query<SsePingParams>,
) -> Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>> {
    let interval = Duration::from_millis(params.interval_ms);

    let stream = stream::repeat_with(|| Ok(Event::default().event("ping").data("ping")))
        .throttle(interval);

    Sse::new(stream).keep_alive(KeepAlive::default())
}
