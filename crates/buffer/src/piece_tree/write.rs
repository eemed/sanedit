use std::{ops::Range, path::Path};

use crate::ReadOnlyPieceTree;

use super::tree::piece::Piece;

enum WriteOperation {
    Extend(usize),
    Truncate(usize),
    Overwrite { source: Piece, target: Range<usize> },
}

pub fn write_to(path: &Path, pt: &ReadOnlyPieceTree, tmp: &Path) {
    // If pt.orig file backed
    // => dependency graph
    // => to write operations
    // => write to file
    //
    // otherwise just use normal impl pt.write_to
    //
    //
    // piece -> orig
}
