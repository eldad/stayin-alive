use std::{
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

use axum::{
    extract::{MatchedPath, Query},
    response::{
        sse::{Event, KeepAlive},
        Sse,
    },
};
use futures::stream::{self, Stream};
use serde::Deserialize;
use tokio_stream::StreamExt as _;

use crate::metrics::{ConnectionGuard, METRIC_SSE_EVENTS_TOTAL};

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

/// Wraps an inner `Stream`, holding a `ConnectionGuard` for the duration of the
/// stream's lifetime.  When the SSE client disconnects, axum drops the stream
/// which drops the guard and decrements the connection gauge.
///
/// Each yielded item also increments the `sse_events_total` counter, labelled
/// by the matched request path.
struct TrackedStream<S> {
    inner: Pin<Box<S>>,
    _guard: ConnectionGuard,
    events_counter: metrics::Counter,
}

impl<S: Stream> Stream for TrackedStream<S> {
    type Item = S::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let poll = self.inner.as_mut().poll_next(cx);
        if let Poll::Ready(Some(_)) = &poll {
            self.events_counter.increment(1);
        }
        poll
    }
}

/// GET /sse-ping — Server-Sent Events endpoint.
///
/// Keeps the connection open and emits a "ping" event at the requested
/// interval (`interval_ms` query parameter, default 1000 ms).
pub async fn sse_ping_handler(
    matched_path: MatchedPath,
    Query(params): Query<SsePingParams>,
) -> Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>> {
    let interval = Duration::from_millis(params.interval_ms);

    let inner = Box::pin(
        stream::repeat_with(|| Ok(Event::default().event("ping").data("ping"))).throttle(interval),
    );

    let events_counter =
        metrics::counter!(METRIC_SSE_EVENTS_TOTAL, "path" => matched_path.as_str().to_owned());

    let stream = TrackedStream {
        inner,
        _guard: ConnectionGuard::new("sse"),
        events_counter,
    };

    Sse::new(stream).keep_alive(KeepAlive::default())
}
