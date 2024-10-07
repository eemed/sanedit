use thiserror::Error;
use tokio::sync::mpsc::error::SendError;

use crate::Response;

#[derive(Error, Debug)]
pub enum LSPSpawnError {
    #[error("No stdout found")]
    Stdout,

    #[error("No stdout found")]
    Stderr,

    #[error("No stdout found")]
    Stdin,

    #[error("LSP initialization failed")]
    Initialize,

    #[error("IO error: {0}")]
    IO(#[from] tokio::io::Error),
}

#[derive(Error, Debug)]
pub enum LSPRequestError {
    #[error("Server does not support this request")]
    Unsupported,

    #[error("Server is closed")]
    ServerClosed,
}

/// Internal error type
#[derive(Error, Debug)]
pub(crate) enum LSPError {
    #[error("Failed spawn LSP server")]
    Spawn(#[from] LSPSpawnError),

    #[error("Faile to receive LSP request")]
    Receive,

    #[error("Failed to send response to client: {0}")]
    Response(#[from] SendError<Response>),

    #[error("No response from LSP")]
    NoResponse,

    #[error("LSP responded with empty response")]
    EmptyResponse,

    #[error("LSP responded to nonexistent request")]
    ResponseToNonexistentRequest,

    #[error("LSP responded with invalid data: {0}")]
    InvalidResponse(String),

    #[error("IO error: {0}")]
    IO(#[from] tokio::io::Error),

    #[error("Json serialization error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Internal channel failure")]
    InternalChannel,
}
