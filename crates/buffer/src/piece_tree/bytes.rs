use std::{io, ops::Range};

use crate::PieceTreeSlice;

use super::{
    chunks::{Chunk, Chunks},
    PieceTreeView,
};

#[derive(Debug, Clone)]
pub struct Bytes<'a> {
    range: Range<u64>,
    chunks: Chunks<'a>,
    chunk: Option<Chunk<'a>>,
    chunk_pos: u64,
    chunk_len: usize, // cache chunk len to improve next performance about from 1.7ns to about 1.1ns ~30%.
    pos: usize,       // Position relative to the current chunk.
}

impl<'a> Bytes<'a> {
    #[inline]
    pub(crate) fn new(pt: &'a PieceTreeView, at: u64) -> Bytes<'a> {
        let chunks = Chunks::new(pt, at);
        let chunk = chunks.get();
        let pos = chunk.as_ref().map(|(pos, _)| at - pos).unwrap_or(0) as usize;
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
    pub(crate) fn new_from_slice(slice: &PieceTreeSlice<'a>, at: u64) -> Bytes<'a> {
        debug_assert!(
            slice.len() >= at,
            "Attempting to index {} over slice len {} ",
            at,
            slice.len(),
        );
        let srange = slice.range.clone();
        let chunks = Chunks::new_from_slice(&slice, at);
        let chunk = chunks.get();
        let chunk_pos = chunk
            .as_ref()
            .map(|(p, _)| *p)
            .unwrap_or(srange.end - srange.start);
        let pos = chunk.as_ref().map(|(pos, _)| at - pos).unwrap_or(0) as usize;
        let chunk = chunk.map(|(_, c)| c);
        let chunk_len = chunk.as_ref().map(|c| c.as_ref().len()).unwrap_or(0);
        Bytes {
            chunk,
            chunks,
            chunk_pos,
            chunk_len,
            pos,
            range: srange,
        }
    }

    #[inline(always)]
    pub(crate) fn get(&mut self) -> Option<u8> {
        if self.pos >= self.chunk_len {
            self.pos = 0;
            let (chunk, pos, len): (Option<Chunk>, u64, usize) = self
                .chunks
                .next()
                .map(|(pos, chunk)| {
                    let len = chunk.as_ref().len();
                    (Some(chunk), pos, len)
                })
                .unwrap_or((None, self.range.end - self.range.start, 0));
            self.chunk = chunk;
            self.chunk_pos = pos;
            self.chunk_len = len;
        }

        let chunk = self.chunk.as_ref()?.as_ref();
        let byte = chunk[self.pos];
        Some(byte)
    }

    #[inline]
    pub fn next(&mut self) -> Option<u8> {
        let byte = self.get()?;
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
    pub fn pos(&self) -> u64 {
        self.chunk_pos + self.pos as u64
    }

    pub fn len(&mut self) -> u64 {
        self.range.end - self.range.start
    }

    /// Get byte at a position. This will iterate the iterator to the specific
    /// byte requested.
    pub fn at(&mut self, pos: u64) -> u8 {
        debug_assert!(
            pos <= self.len(),
            "Indexing out of slice: pos {pos}, slice len: {}",
            self.len(),
        );

        // TODO: if the requested position is far away from current position,
        // instead of scrolling to it, just recreate self?
        while self.pos() < pos {
            self.next();
        }

        while self.pos() > pos {
            self.prev();
        }

        self.get().unwrap()
    }
}

impl<'a> io::Read for Bytes<'a> {
    fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
        todo!()
    }
}

#[cfg(test)]
mod test {
    use crate::PieceTree;

    fn as_byte(string: &str) -> Option<u8> {
        Some(string.as_bytes()[0])
    }

    #[test]
    fn bytes_empty() {
        let pt = PieceTree::new();
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
