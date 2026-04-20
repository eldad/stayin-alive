use axum::response::IntoResponse;

/// GET /ping — simple pong reply.
pub async fn ping_handler() -> impl IntoResponse {
    "pong"
}
