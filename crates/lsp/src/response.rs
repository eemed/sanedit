use std::path::PathBuf;

use crate::Position;

#[derive(Debug, Clone)]
pub enum Response {
    Request(RequestResult),
    // Notification(),
}

#[derive(Debug, Clone)]
pub enum RequestResult {
    Hover {
        text: String,
        position: Position,
    },
    GotoDefinition {
        path: PathBuf,
        position: Position,
    },
    Complete {
        path: PathBuf,
        position: Position,
        results: Vec<String>,
    },
}
