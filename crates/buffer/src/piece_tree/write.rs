use std::{io, ops::Range, path::Path};

use crate::{piece_tree::buffers::BufferKind, ReadOnlyPieceTree};

use super::tree::{
    piece::{self, Piece},
    pieces::Pieces,
};

enum WriteOp {
    ExtendFileTo { size: usize },
    TruncateFileTo { size: usize },
    Overwrite,
}

struct Overwrite {
    piece: Piece,
    source: usize,
    target: usize,
}

pub fn write_in_place(pt: &ReadOnlyPieceTree) -> io::Result<()> {
    if !pt.is_file_backed() {
        todo!()
    }

    todo!()

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

fn calculate_write_operations(pt: &ReadOnlyPieceTree) -> Vec<WriteOp> {
    let mut pcs = Vec::new();
    let mut pieces = Pieces::new(pt, 0);
    while let Some((pos, piece)) = pieces.next() {
        if piece.kind == BufferKind::Original && pt.orig.is_in_file(pos, &piece) {
            continue;
        }

        let target = pos..pos + piece.len;
        pcs.push(Overwrite {
            source: piece,
            target,
        })
    }

    todo!()
}
