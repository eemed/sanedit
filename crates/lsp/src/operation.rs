use std::path::PathBuf;

use sanedit_buffer::ReadOnlyPieceTree;

#[derive(Debug, Clone)]
pub enum Operation {
    DidOpen {
        path: PathBuf,
        buf: ReadOnlyPieceTree,
    },
    Hover {
        path: PathBuf,
        buf: ReadOnlyPieceTree,
        offset: usize,
    },
}
