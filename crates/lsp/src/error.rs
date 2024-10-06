use thiserror::Error;
use tokio::sync::mpsc::error::SendError;

use crate::Response;

#[derive(Error, Debug)]
pub enum LSPError {
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

    #[error("LSP initialization failed")]
    Initialize,

    #[error("Internal channel failure")]
    InternalChannel,
}
