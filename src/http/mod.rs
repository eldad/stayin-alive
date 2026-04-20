use axum::{routing::get, Router};

mod ping;
mod sse;
mod ws;

pub use ping::ping_handler;
pub use sse::sse_ping_handler;
pub use ws::ws_handler;

/// Build the HTTP router with all keepalive endpoints.
pub fn router() -> Router {
    Router::new()
        .route("/ping", get(ping_handler))
        .route("/sse-ping", get(sse_ping_handler))
        .route("/ws", get(ws_handler))
}
