use std::path::PathBuf;

use sanedit_buffer::ReadOnlyPieceTree;

#[derive(Debug, Clone)]
pub enum Request {
    DidOpen {
        path: PathBuf,
        buf: ReadOnlyPieceTree,
    },
    Hover {
        path: PathBuf,
        buf: ReadOnlyPieceTree,
        offset: u64,
    },
    GotoDefinition {
        path: PathBuf,
        buf: ReadOnlyPieceTree,
        offset: u64,
    },
}
