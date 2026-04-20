use std::time::Duration;

use tonic::{Request, Response, Status};

use crate::proto::stayin_alive_server::StayinAlive;
use crate::proto::{
    PingMeLaterRequest, PingMeLaterResponse, PingRequest, PingResponse,
};

#[derive(Debug, Default)]
pub struct StayinAliveService;

#[tonic::async_trait]
impl StayinAlive for StayinAliveService {
    /// Simple ping: returns immediately with a pong message.
    async fn ping(
        &self,
        _request: Request<PingRequest>,
    ) -> Result<Response<PingResponse>, Status> {
        Ok(Response::new(PingResponse {
            message: "pong".to_string(),
        }))
    }

    /// Delayed ping: waits `delay_ms` milliseconds, then returns.
    async fn ping_me_later(
        &self,
        request: Request<PingMeLaterRequest>,
    ) -> Result<Response<PingMeLaterResponse>, Status> {
        let delay_ms = request.into_inner().delay_ms;
        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
        Ok(Response::new(PingMeLaterResponse {
            message: "pong".to_string(),
        }))
    }
}
