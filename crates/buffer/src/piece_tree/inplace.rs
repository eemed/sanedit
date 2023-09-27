use std::{fmt, io, ops::Range};

use crate::{
    piece_tree::{buffers::BufferKind, tree::pieces::PieceIter},
    ReadOnlyPieceTree,
};

use super::tree::piece::Piece;

#[derive(Debug)]
enum Write {
    Extend(usize),
    Truncate(usize),
    Overwrite(Overwrite),
}

#[derive(PartialEq)]
struct Overwrite {
    piece: Piece,
    target: usize,
}

impl fmt::Debug for Overwrite {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} => {:?}", self.depends_on(), self.target())
    }
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

/// Writes a file backed piece tree in place.
///
/// Good:
///      1. If only replaced or appended bytes, saving will be very fast
///      2. No need for additional disk space as no copy is created
///
/// Bad:
///      1. If io error occurs while saving the file will be left in an
///         incomplete state
///      2. Probably slower than writing a copy if insert/remove operations are
///         in the beginning portion of the file
///      3. Previously created undo points/marks cannot be used anymore
pub fn write_in_place(pt: &ReadOnlyPieceTree) -> io::Result<()> {
    if !pt.is_file_backed() {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "piecetree is not file backed",
        ));
    }

    let ops = in_place_write_ops(pt);
    do_write_in_place(ops)
}

fn find_non_depended_target(ows: &Vec<Overwrite>) -> usize {
    for (i, ow) in ows.iter().enumerate() {
        let mut good = true;

        for (j, other) in ows.iter().enumerate() {
            let Range { start, end } = ow.target();
            let Range {
                start: dstart,
                end: dend,
            } = other.depends_on().unwrap();

            if i != j && start < dend && dstart < end {
                good = false;
                break;
            }
        }

        if good {
            return i;
        }
    }

    unreachable!("Cannot find a overwrite with target that does not overlap with other overwrites dependencies");
}

fn in_place_write_ops(pt: &ReadOnlyPieceTree) -> Vec<Write> {
    let mut adds = Vec::with_capacity(pt.piece_count());
    let mut origs = Vec::with_capacity(pt.piece_count());
    let mut iter = PieceIter::new(pt, 0);
    let mut ppiece = iter.get();

    while let Some((pos, piece)) = ppiece {
        match piece.kind {
            BufferKind::Add => adds.push(Overwrite { piece, target: pos }),
            BufferKind::Original => {
                if piece.pos != pos {
                    origs.push(Overwrite { piece, target: pos });
                }
            }
        }

        ppiece = iter.next();
    }

    let olen = pt.orig.len();
    let nlen = pt.len();

    let mut result = Vec::with_capacity(pt.piece_count());
    if olen < nlen {
        result.push(Write::Extend(nlen))
    }

    // Sort the results, so that targets do not step on dependencies
    while !origs.is_empty() {
        let pos = find_non_depended_target(&origs);
        let ow = origs.remove(pos);
        result.push(Write::Overwrite(ow));
    }

    result.extend(adds.into_iter().map(|item| Write::Overwrite(item)));

    if nlen < olen {
        result.push(Write::Truncate(nlen))
    }

    result
}

fn do_write_in_place(ops: Vec<Write>) -> io::Result<()> {
    for op in ops {
        use Write::*;
        match op {
            Extend(size) => todo!(),
            Truncate(size) => todo!(),
            Overwrite(ow) => todo!(),
        }
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::PieceTree;
    use std::{fs::File, path::PathBuf};

    #[test]
    fn write_ops() {
        let path = PathBuf::from("../../test-files/lorem.txt");
        let file = File::open(&path).unwrap();
        let mut pt = PieceTree::mmap(file).unwrap();
        // pt.insert(0, "a");
        // pt.remove(0..10);
        pt.insert(60, "abba");
        pt.insert(30, "a");
        pt.remove(35..40);
        pt.insert(70, "a");
        let ows = in_place_write_ops(&pt.pt);

        for ow in ows {
            println!("{ow:?}",);
        }
    }
}
