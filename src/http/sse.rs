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

use crate::metrics::{
    ConnectionGuard, METRIC_SSE_ERRORS_TOTAL, METRIC_SSE_EVENTS_TOTAL,
    METRIC_SSE_EVENT_BYTES_TOTAL,
};

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

/// Holds SSE event metadata so metrics can be recorded before converting
/// to an opaque [`Event`].
struct SseEventData {
    event_type: String,
    data: String,
}

impl SseEventData {
    fn into_axum_event(self) -> Event {
        Event::default().event(self.event_type).data(self.data)
    }
}

/// Wraps an inner `Stream` of [`SseEventData`], holding a `ConnectionGuard`
/// for the duration of the stream's lifetime.  When the SSE client
/// disconnects, axum drops the stream which drops the guard and decrements
/// the connection gauge.
///
/// Each yielded item is inspected to record metrics:
/// * Non-error events increment `sse_events_total`.
/// * Error events (event type `"error"`) increment `sse_errors_total`.
/// * The data payload length is always added to `sse_event_bytes_total`.
///
/// All counters are labelled by request `path` and `event` type.
struct TrackedStream<S> {
    inner: Pin<Box<S>>,
    _guard: ConnectionGuard,
    path: String,
}

impl<S: Stream<Item = SseEventData>> Stream for TrackedStream<S> {
    type Item = Result<Event, std::convert::Infallible>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.inner.as_mut().poll_next(cx) {
            Poll::Ready(Some(event_data)) => {
                let labels = [
                    ("path", self.path.clone()),
                    ("event", event_data.event_type.clone()),
                ];

                if event_data.event_type == "error" {
                    metrics::counter!(METRIC_SSE_ERRORS_TOTAL, &labels).increment(1);
                } else {
                    metrics::counter!(METRIC_SSE_EVENTS_TOTAL, &labels).increment(1);
                }

                metrics::counter!(METRIC_SSE_EVENT_BYTES_TOTAL, &labels)
                    .increment(event_data.data.len() as u64);

                Poll::Ready(Some(Ok(event_data.into_axum_event())))
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
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
        stream::repeat_with(|| SseEventData {
            event_type: "ping".into(),
            data: "ping".into(),
        })
        .throttle(interval),
    );

    let stream = TrackedStream {
        inner,
        _guard: ConnectionGuard::new("sse"),
        path: matched_path.as_str().to_owned(),
    };

    Sse::new(stream).keep_alive(KeepAlive::default())
}
