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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use http_body_util::BodyExt;
    use tokio::net::TcpListener;
    use tower::ServiceExt;

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
        let app = router();
        let resp = app.oneshot(get_request("/ping")).await.unwrap();

        assert_eq!(resp.status(), 200);
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(&body[..], b"pong");
    }

    #[tokio::test]
    async fn ping_wrong_method_returns_405() {
        let app = router();
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
        let app = router();
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
        let app = router();
        // No interval_ms param — should still succeed with default.
        let resp = app.oneshot(get_request("/sse-ping")).await.unwrap();
        assert_eq!(resp.status(), 200);
    }

    // ---- /ws ----

    #[tokio::test]
    async fn ws_upgrade_requires_websocket_headers() {
        // A plain GET without upgrade headers should be rejected.
        let app = router();
        let resp = app.oneshot(get_request("/ws")).await.unwrap();
        assert_eq!(resp.status(), 400);
    }

    #[tokio::test]
    async fn ws_sends_ping_messages() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            axum::serve(listener, router()).await.unwrap();
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

    // ---- 404 ----

    #[tokio::test]
    async fn unknown_route_returns_404() {
        let app = router();
        let resp = app.oneshot(get_request("/nope")).await.unwrap();
        assert_eq!(resp.status(), 404);
    }
}
