use std::ops::{RangeBounds, Range};

use super::{
    buffers::{BufferKind, ByteSlice},
    tree::{piece::Piece, pieces::Pieces},
    CursorIterator, PieceTree,
};

// Limit max chunk size to not read massive chunks into memory
const MAX_CHUNK_SIZE: usize = 1024 * 64;

#[derive(Debug, Clone, PartialEq)]
pub struct Chunk<'a>(pub(crate) ByteSlice<'a>);

impl<'a> AsRef<[u8]> for Chunk<'a> {
    #[inline(always)]
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
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
    pos: usize,
}

impl<'a> Chunks<'a> {
    #[inline]
    pub fn new(pt: &'a PieceTree, at: usize) -> Chunks<'a> {
        let pieces = Pieces::new(pt, at);
        let pos = pieces.pos();
        Chunks { pt, pieces, pos }
    }
}

impl<'a> CursorIterator for Chunks<'a> {
    type Item = Chunk<'a>;

    #[inline]
    fn pos(&self) -> usize {
        self.pos
    }

    #[inline]
    fn get(&self) -> Option<Chunk<'a>> {
        let piece = self.pieces.get()?;
        read_piece(&self.pt, &piece)
    }

    #[inline]
    fn next(&mut self) -> Option<Chunk<'a>> {
        let piece = self.pieces.get()?;
        self.pos += piece.len;
        let p_pos = self.pieces.pos();
        let end = p_pos + piece.len;

        let piece = if end <= self.pos {
            self.pieces.next()?
        } else {
            self.pieces.get()?
        };

        read_piece(&self.pt, &piece)
    }

    #[inline]
    fn prev(&mut self) -> Option<Chunk<'a>> {
        let p_pos = self.pieces.pos();
        let piece = if self.pos == p_pos {
            self.pieces.prev()?
        } else {
            self.pieces.get()?
        };

        self.pos -= piece.len;
        read_piece(&self.pt, &piece)
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

    fn chunk(string: &str) -> Option<Chunk> {
        let bytes = ByteSlice::Memory {
            bytes: string.as_ref(),
        };
        Some(Chunk(bytes))
    }

    #[test]
    fn next_start() {
        let mut pt = PieceTree::new();
        pt.insert_str(0, "bar");
        pt.insert_str(0, "foo");

        let mut chunks = pt.chunks();

        assert_eq!(chunk("foo"), chunks.get());
        assert_eq!(0, chunks.pos());

        assert_eq!(chunk("bar"), chunks.next());
        assert_eq!(3, chunks.pos());

        assert_eq!(None, chunks.next());
        assert_eq!(6, chunks.pos());

        assert_eq!(None, chunks.next());
        assert_eq!(None, chunks.next());
        assert_eq!(6, chunks.pos());
    }

    #[test]
    fn prev_end() {
        let mut pt = PieceTree::new();
        pt.insert_str(0, "bar");
        pt.insert_str(0, "foo");

        let mut chunks = pt.chunks_at(pt.len);

        assert_eq!(None, chunks.get());
        assert_eq!(6, chunks.pos());

        assert_eq!(chunk("bar"), chunks.prev());
        assert_eq!(3, chunks.pos());

        assert_eq!(chunk("foo"), chunks.prev());
        assert_eq!(0, chunks.pos());

        assert_eq!(None, chunks.prev());
        assert_eq!(0, chunks.pos());
        assert_eq!(chunk("foo"), chunks.get());
    }

    #[test]
    fn next_prev() {
        let mut pt = PieceTree::new();
        pt.insert_str(0, "bar");
        pt.insert_str(0, "foo");

        let mut chunks = pt.chunks();

        assert_eq!(chunk("foo"), chunks.get());
        assert_eq!(0, chunks.pos());

        assert_eq!(chunk("bar".as_ref()), chunks.next());
        assert_eq!(3, chunks.pos());

        assert_eq!(chunk("foo"), chunks.prev());
        assert_eq!(0, chunks.pos());
    }

    #[test]
    fn prev_next() {
        let mut pt = PieceTree::new();
        pt.insert_str(0, "bar");
        pt.insert_str(0, "foo");

        let mut chunks = pt.chunks_at(pt.len);

        assert_eq!(None, chunks.get());
        assert_eq!(6, chunks.pos());

        assert_eq!(chunk("bar"), chunks.prev());
        assert_eq!(3, chunks.pos());

        assert_eq!(chunk("foo"), chunks.prev());
        assert_eq!(0, chunks.pos());

        assert_eq!(chunk("bar"), chunks.next());
        assert_eq!(3, chunks.pos());
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
