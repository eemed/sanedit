use std::ops::{Range, RangeBounds};

use super::{
    buffers::{BufferKind, ByteSlice},
    tree::{piece::Piece, pieces::Pieces},
    PieceTree,
};

#[derive(Debug, Clone, PartialEq)]
pub struct Chunk<'a>(pub(crate) ByteSlice<'a>);

impl<'a> AsRef<[u8]> for Chunk<'a> {
    #[inline(always)]
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl<'a> From<&[u8]> for Chunk<'a> {
    fn from(_: &[u8]) -> Self {
        todo!()
    }
}

impl<'a> Chunk<'a> {
    pub fn slice<R: RangeBounds<usize>>(&self, range: R) -> Chunk<'a> {
        Chunk(self.0.slice(range))
    }
}

#[derive(Debug, Clone)]
pub struct Chunks<'a> {
    pt: &'a PieceTree,
    pieces: Pieces<'a>,
}

impl<'a> Chunks<'a> {
    #[inline]
    pub fn new(pt: &'a PieceTree, at: usize) -> Chunks<'a> {
        let pieces = Pieces::new(pt, at);
        Chunks { pt, pieces }
    }

    #[inline]
    pub fn new_from_slice(pt: &'a PieceTree, at: usize, range: Range<usize>) -> Chunks<'a> {
        let pieces = Pieces::new_from_slice(pt, at, range);
        Chunks { pt, pieces }
    }

    #[inline]
    pub fn get(&self) -> Option<(usize, Chunk<'a>)> {
        let (p_pos, piece) = self.pieces.get()?;
        let chunk = read_piece(&self.pt, &piece)?;
        Some((p_pos, chunk))
    }

    #[inline]
    pub fn next(&mut self) -> Option<(usize, Chunk<'a>)> {
        let (p_pos, piece) = self.pieces.next()?;
        let chunk = read_piece(&self.pt, &piece)?;
        Some((p_pos, chunk))
    }

    #[inline]
    pub fn prev(&mut self) -> Option<(usize, Chunk<'a>)> {
        let (p_pos, piece) = self.pieces.prev()?;
        let chunk = read_piece(&self.pt, &piece)?;
        Some((p_pos, chunk))
    }
}

#[inline(always)]
fn read_piece<'a>(pt: &'a PieceTree, piece: &Piece) -> Option<Chunk<'a>> {
    match piece.kind {
        BufferKind::Add => Some(Chunk(ByteSlice::Memory {
            bytes: &pt.add[piece.pos..piece.pos + piece.len],
        })),
        BufferKind::Original => Some(Chunk(pt.orig.slice(piece.pos..piece.pos + piece.len).ok()?)),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn chunk(pos: usize, string: &str) -> Option<(usize, Chunk)> {
        let bytes = ByteSlice::Memory {
            bytes: string.as_ref(),
        };
        Some((pos, Chunk(bytes)))
    }

    #[test]
    fn next_start() {
        let mut pt = PieceTree::new();
        pt.insert_str(0, "bar");
        pt.insert_str(0, "foo");

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
        pt.insert_str(0, "bar");
        pt.insert_str(0, "foo");

        let mut chunks = pt.chunks_at(pt.len);

        assert_eq!(None, chunks.get());

        assert_eq!(chunk(3, "bar"), chunks.prev());
        assert_eq!(chunk(0, "foo"), chunks.prev());

        assert_eq!(None, chunks.prev());
        assert_eq!(chunk(0, "foo"), chunks.get());
    }

    #[test]
    fn next_prev() {
        let mut pt = PieceTree::new();
        pt.insert_str(0, "bar");
        pt.insert_str(0, "foo");

        let mut chunks = pt.chunks();

        assert_eq!(chunk(0, "foo"), chunks.get());
        assert_eq!(chunk(3, "bar"), chunks.next());
        assert_eq!(chunk(0, "foo"), chunks.prev());
    }

    #[test]
    fn prev_next() {
        let mut pt = PieceTree::new();
        pt.insert_str(0, "bar");
        pt.insert_str(0, "foo");

        let mut chunks = pt.chunks_at(pt.len);

        assert_eq!(None, chunks.get());
        assert_eq!(chunk(3, "bar"), chunks.prev());
        assert_eq!(chunk(0, "foo"), chunks.prev());
        assert_eq!(chunk(3, "bar"), chunks.next());
    }

    // #[test]
    // fn over_max_chunk_size() {
    //     let mut pt = PieceTree::new();
    //     pt.insert_str(0, &"a".repeat(MAX_CHUNK_SIZE * 2));
    //     let chunks = pt.chunks_at(15);

    //     assert_eq!(15, chunks.pos());
    //     assert_eq!(chunk(&"a".repeat(MAX_CHUNK_SIZE)), chunks.get());
    // }
}
