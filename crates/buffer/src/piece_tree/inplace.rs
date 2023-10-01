use std::{
    cmp::{max, min},
    fmt,
    fs::File,
    io::{self, Read, Seek, SeekFrom, Write},
    ops::Range,
    path::Path,
};

use crate::{
    piece_tree::{buffers::BufferKind, tree::pieces::PieceIter},
    ReadOnlyPieceTree,
};

use super::tree::piece::Piece;

#[derive(Debug)]
enum WriteOp {
    Size(usize),
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

pub fn write_in_place(pt: &ReadOnlyPieceTree) -> io::Result<()> {
    if !pt.is_file_backed() {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "piecetree is not file backed",
        ));
    }

    let ops = in_place_write_ops(pt);
    do_write_in_place(pt, ops)
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

fn in_place_write_ops(pt: &ReadOnlyPieceTree) -> Vec<WriteOp> {
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
        result.push(WriteOp::Size(nlen))
    }

    // Sort the results, so that targets do not step on dependencies
    while !origs.is_empty() {
        let pos = find_non_depended_target(&origs);
        let ow = origs.remove(pos);
        result.push(WriteOp::Overwrite(ow));
    }

    result.extend(adds.into_iter().map(|item| WriteOp::Overwrite(item)));

    if nlen < olen {
        result.push(WriteOp::Size(nlen))
    }

    result
}

fn do_write_in_place(pt: &ReadOnlyPieceTree, ops: Vec<WriteOp>) -> io::Result<()> {
    let mut iter = ops.into_iter();
    let mut op = iter.next();
    let path = pt.orig.file_path();

    while op.is_some() {
        // Handle extending and truncating
        while let Some(WriteOp::Size(size)) = op {
            let file = File::options().append(true).open(path)?;
            file.set_len(size as u64)?;
            op = iter.next();
        }

        // Handle overwriting the file
        let mut file = None;
        while let Some(WriteOp::Overwrite(ow)) = op {
            if file.is_none() {
                file = Some(File::options().read(true).write(true).open(path)?);
            }

            let file = file.as_mut().unwrap();

            match ow.kind() {
                BufferKind::Add => {
                    let start = ow.target().start;
                    file.seek(SeekFrom::Start(start as u64))?;
                    let range = ow.piece.range();
                    let bytes = pt.add.slice(range);
                    file.write_all(bytes)?;
                }
                BufferKind::Original => {
                    let deps = ow.depends_on().unwrap();
                    let target = ow.target();
                    let overlaps = deps.start < target.end && target.start < deps.end;

                    if overlaps {
                        const SIZE: usize = 1024 * 128;
                        let mut buf = [0; SIZE];
                        let mut nbuf;
                        let up = target.start < deps.start;
                        // Deps len == target len
                        let len = target.len();
                        let mut written = 0;

                        // TODO use mmap that already exists?
                        if up {
                            // Moving chunk up
                            let mut rpos = deps.start;
                            let mut wpos = target.start;

                            while written < target.len() {
                                file.seek(SeekFrom::Start(rpos as u64))?;
                                let bmax = min(deps.len() - written, SIZE);
                                nbuf = file.read(&mut buf[..bmax])?;
                                rpos += nbuf;

                                file.seek(SeekFrom::Start(wpos as u64))?;
                                file.write(&mut buf[..nbuf])?;
                                wpos += nbuf;
                                written += nbuf;
                            }
                        } else {
                            // Moving chunk down
                            let mut rpos = deps.end;
                            let mut wpos = target.end;

                            while written < len {
                                let bmax = min(len - written, SIZE);
                                file.seek(SeekFrom::Start((rpos - bmax) as u64))?;
                                nbuf = file.read(&mut buf[..bmax])?;
                                rpos -= nbuf;

                                file.seek(SeekFrom::Start((wpos - nbuf) as u64))?;
                                file.write(&mut buf[..nbuf])?;
                                wpos -= nbuf;
                                written += nbuf;
                            }
                        }
                    } else {
                        file.seek(SeekFrom::Start(target.start as u64))?;
                        let range = ow.piece.range();
                        let bytes = pt.orig.slice(range);
                        file.write_all(bytes.as_ref())?;
                    }
                }
            }

            op = iter.next();
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
        let mut pt = PieceTree::mmap(path).unwrap();
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
