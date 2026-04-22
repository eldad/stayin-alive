use std::time::Duration;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query,
    },
    response::IntoResponse,
};
use serde::Deserialize;

use crate::metrics::ConnectionGuard;

/// Query parameters accepted by the WebSocket endpoint.
#[derive(Debug, Deserialize)]
pub struct WsPingParams {
    /// Interval between consecutive ping messages, in milliseconds.
    /// Defaults to 1000 ms if not specified.
    #[serde(default = "default_interval_ms")]
    interval_ms: u64,
}

fn default_interval_ms() -> u64 {
    1000
}

/// GET /ws — WebSocket endpoint.
///
/// Upgrades the connection to WebSocket and sends a "ping" text message at
/// the requested interval (`interval_ms` query parameter, default 1000 ms).
/// The connection stays open until the client disconnects.
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<WsPingParams>,
) -> impl IntoResponse {
    let interval = Duration::from_millis(params.interval_ms);
    ws.on_upgrade(move |socket| handle_socket(socket, interval))
}

async fn handle_socket(mut socket: WebSocket, interval: Duration) {
    let _guard = ConnectionGuard::new("websocket");

    let mut ticker = tokio::time::interval(interval);
    // The first tick fires immediately; consume it so the first message is
    // sent after one full interval.
    ticker.tick().await;

    loop {
        ticker.tick().await;

        if socket.send(Message::Text("ping".into())).await.is_err() {
            // Client disconnected.
            break;
        }
    }
}
