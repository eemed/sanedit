use std::ops::{Index, Range};

use super::{
    chunks::{Chunk, Chunks},
    PieceTree,
};

#[derive(Debug, Clone)]
pub struct Bytes<'a> {
    range: Range<usize>,
    chunks: Chunks<'a>,
    chunk: Option<Chunk<'a>>,
    chunk_pos: usize,
    chunk_len: usize, // cache chunk len to improve next performance about from 1.7ns to about 1.1ns ~30%.
    pos: usize,       // Position relative to the current chunk.
}

impl<'a> Bytes<'a> {
    #[inline]
    pub(crate) fn new(pt: &'a PieceTree, at: usize) -> Bytes<'a> {
        let chunks = Chunks::new(pt, at);
        let chunk = chunks.get();
        let pos = chunk.as_ref().map(|(pos, _)| at - pos).unwrap_or(0);
        let chunk_pos = chunk.as_ref().map(|(p, _)| *p).unwrap_or(pt.len());
        let chunk = chunk.map(|(_, c)| c);
        let chunk_len = chunk.as_ref().map(|c| c.as_ref().len()).unwrap_or(0);
        Bytes {
            chunk,
            chunks,
            chunk_pos,
            chunk_len,
            pos,
            range: 0..pt.len(),
        }
    }

    #[inline]
    pub(crate) fn new_from_slice(pt: &'a PieceTree, at: usize, range: Range<usize>) -> Bytes<'a> {
        debug_assert!(
            range.end - range.start >= at,
            "Attempting to index {} over slice len {} ",
            at,
            range.end - range.start,
        );
        let chunks = Chunks::new_from_slice(pt, at, range.clone());
        let chunk = chunks.get();
        let chunk_pos = chunk.as_ref().map(|(p, _)| *p).unwrap_or(range.len());
        let pos = chunk.as_ref().map(|(pos, _)| at - pos).unwrap_or(0);
        let chunk = chunk.map(|(_, c)| c);
        let chunk_len = chunk.as_ref().map(|c| c.as_ref().len()).unwrap_or(0);
        Bytes {
            chunk,
            chunks,
            chunk_pos,
            chunk_len,
            pos,
            range,
        }
    }

    #[inline]
    pub fn next(&mut self) -> Option<u8> {
        if self.pos >= self.chunk_len {
            self.pos = 0;
            let (chunk, pos, len): (Option<Chunk>, usize, usize) = self
                .chunks
                .next()
                .map(|(pos, chunk)| {
                    let len = chunk.as_ref().len();
                    (Some(chunk), pos, len)
                })
                .unwrap_or((None, self.range.len(), 0));
            self.chunk = chunk;
            self.chunk_pos = pos;
            self.chunk_len = len;
        }

        let chunk = self.chunk.as_ref()?.as_ref();
        let byte = chunk[self.pos];
        self.pos += 1;
        Some(byte)
    }

    #[inline]
    pub fn prev(&mut self) -> Option<u8> {
        if self.pos != 0 {
            self.pos -= 1;
        } else {
            let (pos, chunk) = self.chunks.prev()?;
            let len = chunk.as_ref().len();
            self.pos = len.saturating_sub(1);
            self.chunk_pos = pos;
            self.chunk_len = len;
            self.chunk = Some(chunk);
        }

        let chunk = self.chunk.as_ref()?.as_ref();
        let byte = chunk[self.pos];
        Some(byte)
    }

    #[inline]
    pub fn pos(&self) -> usize {
        self.chunk_pos + self.pos
    }

    pub fn byte_at(&mut self, pos: usize) -> u8 {
        let spos = self.pos();

        // If currently on position
        if spos == pos {
            return self.next().unwrap();
        }

        // If previous byte is the one we need
        if spos != 0 && spos - 1 == pos {
            return self.prev().unwrap();
        }

        while self.pos() < pos {
            self.next();
        }

        while self.pos() > pos {
            self.prev();
        }

        self.next().unwrap()
    }
}

impl<'a> Index<usize> for Bytes<'a> {
    type Output = u8;

    fn index(&self, index: usize) -> &Self::Output {
        todo!()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn as_byte(string: &str) -> Option<u8> {
        Some(string.as_bytes()[0])
    }

    #[test]
    fn bytes_empty() {
        let mut pt = PieceTree::new();
        let mut bytes = pt.bytes();
        assert_eq!(None, bytes.next());
    }

    #[test]
    fn bytes_next() {
        let mut pt = PieceTree::new();
        pt.insert(0, "foo");
        let mut bytes = pt.bytes();

        assert_eq!(as_byte("f"), bytes.next());
        assert_eq!(as_byte("o"), bytes.next());
        assert_eq!(as_byte("o"), bytes.next());

        assert!(bytes.next().is_none());
        assert!(bytes.next().is_none());
    }

    #[test]
    fn bytes_prev() {
        let mut pt = PieceTree::new();
        pt.insert(0, "foo");
        let mut bytes = pt.bytes_at(pt.len());

        assert_eq!(bytes.pos(), pt.len());
        assert!(bytes.next().is_none());
        assert_eq!(as_byte("o"), bytes.prev());
        assert_eq!(2, bytes.pos());
        assert_eq!(as_byte("o"), bytes.prev());
        assert_eq!(1, bytes.pos());
        assert_eq!(as_byte("f"), bytes.prev());
        assert_eq!(0, bytes.pos());
        assert!(bytes.prev().is_none());
        assert!(bytes.prev().is_none());
        assert!(bytes.prev().is_none());

        assert_eq!(0, bytes.pos());
        assert_eq!(as_byte("f"), bytes.next());
    }

    #[test]
    fn bytes_back_and_forth() {
        let mut pt = PieceTree::new();
        pt.insert(0, "foo");
        let mut bytes = pt.bytes();

        assert_eq!(Some(b'f'), bytes.next());
        assert_eq!(as_byte("o"), bytes.next());
        assert_eq!(as_byte("o"), bytes.next());
        assert_eq!(None, bytes.next());
        assert_eq!(None, bytes.next());
        assert_eq!(None, bytes.next());
        assert!(bytes.next().is_none());
        assert_eq!(as_byte("o"), bytes.prev());
        assert_eq!(as_byte("o"), bytes.prev());
        assert_eq!(as_byte("f"), bytes.prev());
        assert_eq!(None, bytes.prev());
        assert_eq!(as_byte("f"), bytes.next());
    }

    #[test]
    fn bytes_start_middle() {
        let mut pt = PieceTree::new();
        pt.insert(0, "bar");
        pt.insert(0, "foo");
        let mut bytes = pt.bytes_at(3);

        assert_eq!(3, bytes.pos());
        assert_eq!(as_byte("o"), bytes.prev());
        assert_eq!(2, bytes.pos());
        assert_eq!(as_byte("o"), bytes.prev());
        assert_eq!(1, bytes.pos());
        assert_eq!(as_byte("f"), bytes.prev());
        assert_eq!(0, bytes.pos());
    }

    #[test]
    fn bytes_slice() {
        let mut pt = PieceTree::new();
        pt.insert(0, "bar");
        pt.insert(0, "foo");
        let slice = pt.slice(2..);
        let mut bytes = slice.bytes();

        assert_eq!(0, bytes.pos());
        assert_eq!(as_byte("o"), bytes.next());
        assert_eq!(as_byte("b"), bytes.next());
        assert_eq!(as_byte("a"), bytes.next());
        assert_eq!(as_byte("r"), bytes.next());
        assert_eq!(slice.len(), bytes.pos());
        assert_eq!(None, bytes.next());
    }

    #[test]
    fn bytes_slice_prev() {
        let mut pt = PieceTree::new();
        pt.insert(0, "bar");
        pt.insert(0, "foo");
        let slice = pt.slice(2..);
        let mut bytes = slice.bytes_at(slice.len());

        assert_eq!(as_byte("r"), bytes.prev());
        assert_eq!(as_byte("a"), bytes.prev());
        assert_eq!(as_byte("b"), bytes.prev());
        assert_eq!(as_byte("o"), bytes.prev());
        assert_eq!(None, bytes.prev());
    }
}
