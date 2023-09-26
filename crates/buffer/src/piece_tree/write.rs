use std::{cmp::Ordering, io, ops::Range, path::Path};

use crate::{
    piece_tree::{buffers::BufferKind, tree::pieces::PieceIter},
    ReadOnlyPieceTree,
};

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
    target: usize,
}

impl Overwrite {
    fn kind(&self) -> BufferKind {
        self.piece.kind
    }

    fn depends_on(&self) -> Option<Range<usize>> {
        match self.piece.kind {
            BufferKind::Add => None,
            BufferKind::Original => {
                let pos = self.piece.pos;
                let len = self.piece.len;
                Some(pos..pos + len)
            }
        }
    }

    fn target(&self) -> Range<usize> {
        self.target..self.target + self.piece.len
    }
}

pub fn write_in_place(pt: &ReadOnlyPieceTree) -> io::Result<()> {
    if !pt.is_file_backed() {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "piecetree is not file backed",
        ));
    }

    // let olen = pt.orig.len();
    // let nlen = pt.len();

    // if olen < nlen {
    //     // extend
    // }

    // if nlen < olen {
    //     // truncate
    // }

    let mut ows = Vec::with_capacity(pt.piece_count());
    let mut iter = PieceIter::new(pt, 0);
    while let Some((pos, piece)) = iter.next() {
        if piece.kind == BufferKind::Original && piece.pos == pos {
            continue;
        }

        ows.push(Overwrite { piece, target: pos });
    }

    ows.sort_by(|a, b| {
        use BufferKind::*;
        match (a.kind(), b.kind()) {
            (Add, Original) => Ordering::Greater,
            (Original, Add) => Ordering::Less,
            (Add, Add) => {
                let apos = a.piece.pos;
                let bpos = b.piece.pos;
                apos.cmp(&bpos)
            }
            (Original, Original) => {}
        }
    });

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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn write_ops() {}
}
