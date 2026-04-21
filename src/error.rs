use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Address parse error: {0}")]
    AddrParse(#[from] std::net::AddrParseError),

    #[error("gRPC transport error: {0}")]
    Tonic(#[from] tonic::transport::Error),
}
