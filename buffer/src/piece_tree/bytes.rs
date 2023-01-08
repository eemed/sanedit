use std::ops::Range;

use super::{
    chunks::{Chunk, Chunks},
    PieceTree,
};

#[derive(Debug, Clone)]
pub struct Bytes<'a> {
    chunks: Chunks<'a>,
    chunk: Option<Chunk<'a>>,
    chunk_len: usize, // cache chunk len to improve next performance about from 1.7ns to about 1.1ns ~30%.
    pos: usize,       // Position relative to the current chunk.
}

impl<'a> Bytes<'a> {
    #[inline]
    pub(crate) fn new(pt: &'a PieceTree, at: usize) -> Bytes<'a> {
        let chunks = Chunks::new(pt, at);
        let pos_chunk = chunks.get();
        let pos = pos_chunk.as_ref().map(|(pos, _)| at - pos).unwrap_or(0);
        let chunk = pos_chunk.map(|(_, c)| c);
        let chunk_len = chunk.as_ref().map(|c| c.as_ref().len()).unwrap_or(0);
        Bytes {
            chunk,
            chunks,
            chunk_len,
            pos,
        }
    }

    #[inline]
    pub(crate) fn new_from_slice(pt: &'a PieceTree, at: usize, range: Range<usize>) -> Bytes<'a> {
        let chunks = Chunks::new_from_slice(pt, at, range);
        let pos_chunk = chunks.get();
        let pos = pos_chunk.as_ref().map(|(pos, _)| at - pos).unwrap_or(0);
        let chunk = pos_chunk.map(|(_, chunk)| chunk);
        let chunk_len = chunk
            .as_ref()
            .map(|chunk| chunk.as_ref().len())
            .unwrap_or(0);
        Bytes {
            chunk,
            chunks,
            chunk_len,
            pos,
        }
    }

    #[inline]
    pub fn next(&mut self) -> Option<u8> {
        if self.pos >= self.chunk_len {
            self.pos = 0;
            let (chunk, len): (Option<Chunk>, usize) = self
                .chunks
                .next()
                .map(|(_, chunk)| {
                    let len = chunk.as_ref().len();
                    (Some(chunk), len)
                })
                .unwrap_or((None, 0));
            self.chunk = chunk;
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
            let chunk = self.chunks.prev()?.1;
            let len = chunk.as_ref().len();
            self.pos = len.saturating_sub(1);
            self.chunk_len = len;
            self.chunk = Some(chunk);
        }

        let chunk = self.chunk.as_ref()?.as_ref();
        let byte = chunk[self.pos];
        Some(byte)
    }

    #[inline]
    pub fn pos(&self) -> usize {
        self.pos
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
        pt.insert_str(0, "foo");
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
        pt.insert_str(0, "foo");
        let mut bytes = pt.bytes_at(pt.len());

        assert!(bytes.next().is_none());
        assert_eq!(as_byte("o"), bytes.prev());
        assert_eq!(as_byte("o"), bytes.prev());
        assert_eq!(as_byte("f"), bytes.prev());
        assert!(bytes.prev().is_none());
        assert!(bytes.prev().is_none());
        assert!(bytes.prev().is_none());
        assert_eq!(as_byte("f"), bytes.next());
    }

    #[test]
    fn bytes_back_and_forth() {
        let mut pt = PieceTree::new();
        pt.insert_str(0, "foo");
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
        pt.insert_str(0, "bar");
        pt.insert_str(0, "foo");
        let mut bytes = pt.bytes_at(3);

        assert_eq!(as_byte("o"), bytes.prev());
        assert_eq!(as_byte("o"), bytes.prev());
        assert_eq!(as_byte("f"), bytes.prev());
    }
}
