use std::path::PathBuf;

use sanedit_buffer::ReadOnlyPieceTree;

use crate::Position;

#[derive(Debug, Clone)]
pub enum Request {
    DidOpen {
        path: PathBuf,
        buf: ReadOnlyPieceTree,
    },
    Hover {
        path: PathBuf,
        buf: ReadOnlyPieceTree,
        position: Position,
    },
    GotoDefinition {
        path: PathBuf,
        buf: ReadOnlyPieceTree,
        position: Position,
    },
    DidChange {
        path: PathBuf,
        buf: ReadOnlyPieceTree,
        changes: Vec<Change>,
    },
    Complete {
        path: PathBuf,
        buf: ReadOnlyPieceTree,
        position: Position,
    },
}

#[derive(Debug, Clone)]
pub struct Change {
    pub start: Position,
    pub end: Position,
    pub text: String,
}
