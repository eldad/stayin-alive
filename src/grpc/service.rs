use std::time::Duration;

use tonic::{Request, Response, Status};

use crate::metrics::ConnectionGuard;
use crate::proto::stayin_alive_server::StayinAlive;
use crate::proto::{PingMeLaterRequest, PingMeLaterResponse, PingRequest, PingResponse};

#[derive(Debug, Default)]
pub struct StayinAliveService;

#[tonic::async_trait]
impl StayinAlive for StayinAliveService {
    /// Simple ping: returns immediately with a pong message.
    async fn ping(&self, _request: Request<PingRequest>) -> Result<Response<PingResponse>, Status> {
        let _guard = ConnectionGuard::new("grpc");
        Ok(Response::new(PingResponse {
            message: "pong".to_string(),
        }))
    }

    /// Delayed ping: waits `delay_ms` milliseconds, then returns.
    async fn ping_me_later(
        &self,
        request: Request<PingMeLaterRequest>,
    ) -> Result<Response<PingMeLaterResponse>, Status> {
        let _guard = ConnectionGuard::new("grpc");
        let delay_ms = request.into_inner().delay_ms;
        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
        Ok(Response::new(PingMeLaterResponse {
            message: "pong".to_string(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::Instant;

    #[tokio::test]
    async fn ping_returns_pong() {
        let svc = StayinAliveService;
        let resp = svc.ping(Request::new(PingRequest {})).await.unwrap();
        assert_eq!(resp.into_inner().message, "pong");
    }

    #[tokio::test]
    async fn ping_me_later_returns_pong_after_delay() {
        let svc = StayinAliveService;
        let delay_ms = 100;
        let start = Instant::now();
        let resp = svc
            .ping_me_later(Request::new(PingMeLaterRequest { delay_ms }))
            .await
            .unwrap();
        let elapsed = start.elapsed();

        assert_eq!(resp.into_inner().message, "pong");
        assert!(
            elapsed >= Duration::from_millis(delay_ms),
            "expected at least {delay_ms}ms delay, got {elapsed:?}"
        );
    }

    #[tokio::test]
    async fn ping_me_later_zero_delay() {
        let svc = StayinAliveService;
        let resp = svc
            .ping_me_later(Request::new(PingMeLaterRequest { delay_ms: 0 }))
            .await
            .unwrap();
        assert_eq!(resp.into_inner().message, "pong");
    }
}
