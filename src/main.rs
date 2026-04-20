use anyhow::Result;
use tonic::transport::Server as TonicServer;
use tracing::info;

mod error;
mod grpc;
mod http;

/// Generated protobuf types and gRPC server/client stubs.
pub mod proto {
    tonic::include_proto!("stayin_alive");
}

use error::AppError;
use grpc::StayinAliveService;
use proto::stayin_alive_server::StayinAliveServer;

/// Default address for the HTTP server.
const HTTP_ADDR: &str = "0.0.0.0:3000";
/// Default address for the gRPC server.
const GRPC_ADDR: &str = "0.0.0.0:50051";

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "stayin_alive=info,tower_http=info".parse().unwrap()),
        )
        .init();

    run().await.map_err(anyhow::Error::from)
}

async fn run() -> Result<(), AppError> {
    let http_addr: std::net::SocketAddr = HTTP_ADDR.parse()?;
    let grpc_addr: std::net::SocketAddr = GRPC_ADDR.parse()?;

    info!("HTTP server listening on {http_addr}");
    info!("gRPC server listening on {grpc_addr}");

    let http_listener = tokio::net::TcpListener::bind(http_addr).await?;
    let http_server = axum::serve(http_listener, http::router());

    let grpc_server = TonicServer::builder()
        .add_service(StayinAliveServer::new(StayinAliveService))
        .serve(grpc_addr);

    // Run both servers concurrently; if either exits, propagate the error.
    tokio::select! {
        result = http_server => result.map_err(AppError::Io),
        result = grpc_server => result.map_err(AppError::Tonic),
    }
}
