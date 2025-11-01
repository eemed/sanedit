use std::ops::Range;

use crate::PieceTreeSlice;

use super::{
    buffers::{BufferKind, ByteSlice},
    tree::{piece::Piece, pieces::Pieces},
};

#[derive(Debug, Clone, PartialEq)]
pub struct Chunk<'a>(pub(crate) ByteSlice<'a>);

impl<'a> AsRef<[u8]> for Chunk<'a> {
    #[inline(always)]
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

#[derive(Debug, Clone)]
pub struct Chunks<'a> {
    pub(crate) pt: &'a PieceTreeSlice,
    pieces: Pieces<'a>,
}

impl<'a> Chunks<'a> {
    #[inline]
    pub(crate) fn new(pt: &'a PieceTreeSlice, at: u64) -> Chunks<'a> {
        let pieces = Pieces::new(pt, at);
        Chunks { pt, pieces }
    }

    #[inline]
    pub fn get(&self) -> Option<(u64, Chunk<'a>)> {
        let (p_pos, piece) = self.pieces.get()?;
        let chunk = read_piece(self.pt, &piece)?;
        Some((p_pos, chunk))
    }

    #[inline]
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Option<(u64, Chunk<'a>)> {
        let (p_pos, piece) = self.pieces.next()?;
        let chunk = read_piece(self.pt, &piece)?;
        Some((p_pos, chunk))
    }

    #[inline]
    pub fn prev(&mut self) -> Option<(u64, Chunk<'a>)> {
        let (p_pos, piece) = self.pieces.prev()?;
        let chunk = read_piece(self.pt, &piece)?;
        Some((p_pos, chunk))
    }

    pub fn slice(&self) -> &'a PieceTreeSlice {
        self.pt
    }

    pub(crate) fn get_bounds(&self) -> &Range<u64> {
        self.pieces.get_bounds()
    }
}

#[inline(always)]
fn read_piece<'a>(pt: &'a PieceTreeSlice, piece: &Piece) -> Option<Chunk<'a>> {
    match piece.kind {
        BufferKind::Add => {
            let bytes = pt
                .add
                .slice(piece.pos as usize..(piece.pos + piece.len) as usize);
            Some(Chunk(bytes.into()))
        }
        BufferKind::Original => {
            let bytes = pt.orig.slice(piece.pos..piece.pos + piece.len).ok()?;
            Some(Chunk(bytes))
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::PieceTree;

    fn chunk<'a>(pos: u64, string: &'a str) -> Option<(u64, Chunk<'a>)> {
        let bytes = string.as_bytes().into();
        Some((pos, Chunk(bytes)))
    }

    #[test]
    fn next_start() {
        let mut pt = PieceTree::new();
        pt.insert(0, "bar");
        pt.insert(0, "foo");

        let mut chunks = pt.chunks();

        assert_eq!(chunk(0, "foo"), chunks.get());
        assert_eq!(chunk(3, "bar"), chunks.next());

        assert_eq!(None, chunks.next());
        assert_eq!(None, chunks.next());
        assert_eq!(None, chunks.next());
    }

    #[test]
    fn prev_end() {
        let mut pt = PieceTree::new();
        pt.insert(0, "bar");
        pt.insert(0, "foo");

        let mut chunks = pt.chunks_at(pt.len());

        assert_eq!(None, chunks.get());

        assert_eq!(chunk(3, "bar"), chunks.prev());
        assert_eq!(chunk(0, "foo"), chunks.prev());

        assert_eq!(None, chunks.prev());
        assert_eq!(chunk(0, "foo"), chunks.get());
    }

    #[test]
    fn next_prev() {
        let mut pt = PieceTree::new();
        pt.insert(0, "bar");
        pt.insert(0, "foo");

        let mut chunks = pt.chunks();

        assert_eq!(chunk(0, "foo"), chunks.get());
        assert_eq!(chunk(3, "bar"), chunks.next());
        assert_eq!(chunk(0, "foo"), chunks.prev());
    }

    #[test]
    fn prev_next() {
        let mut pt = PieceTree::new();
        pt.insert(0, "bar");
        pt.insert(0, "foo");

        let mut chunks = pt.chunks_at(pt.len());

        assert_eq!(None, chunks.get());
        assert_eq!(chunk(3, "bar"), chunks.prev());
        assert_eq!(chunk(0, "foo"), chunks.prev());
        assert_eq!(chunk(3, "bar"), chunks.next());
    }
}
