use std::path::PathBuf;

use crate::Position;

#[derive(Debug, Clone)]
pub enum Response {
    Request(RequestResult),
    // Notification(),
}

#[derive(Debug, Clone)]
pub enum RequestResult {
    Hover { text: String, offset: u64 },
    GotoDefinition { path: PathBuf, position: Position },
}
