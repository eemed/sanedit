use super::{
    chunks::{Chunk, Chunks},
    CursorIterator, PieceTree,
};

#[derive(Debug, Clone)]
pub struct Bytes<'a> {
    chunks: Chunks<'a>,
    chunk: Option<Chunk<'a>>,
    chunk_len: usize, // cache chunk len to improve next performance about from 1.7ns to about 1.1ns ~30%.
    pos: usize,       // Position relative to the current chunk.
}

impl<'a> Bytes<'a> {
    pub(crate) fn new(pt: &'a PieceTree, at: usize) -> Bytes<'a> {
        let chunks = Chunks::new(pt, at);
        let chunk = chunks.get();
        let chunk_len = chunk.as_ref().map(|c| c.as_ref().len()).unwrap_or(0);
        let pos = at - chunks.pos();
        Bytes {
            chunk,
            chunks,
            chunk_len,
            pos,
        }
    }
}

impl<'a> CursorIterator for Bytes<'a> {
    type Item = u8;

    #[inline]
    fn get(&self) -> Option<u8> {
        let chunk = self.chunk.as_ref()?.as_ref();
        Some(chunk[self.pos])
    }

    #[inline]
    fn pos(&self) -> usize {
        if self.chunk.is_none() {
            self.chunks.pos()
        } else {
            self.chunks.pos() + self.pos
        }
    }

    #[inline]
    fn next(&mut self) -> Option<u8> {
        self.pos += 1;

        if self.pos >= self.chunk_len {
            self.pos = 0;
            self.chunk = self.chunks.next();
            self.chunk_len = self.chunk.as_ref().map(|c| c.as_ref().len()).unwrap_or(0);
        }

        self.get()
    }

    #[inline]
    fn prev(&mut self) -> Option<u8> {
        if self.pos != 0 {
            self.pos -= 1;
        } else {
            let chunk = self.chunks.prev()?;
            self.pos = chunk.as_ref().len().saturating_sub(1);
            self.chunk_len = chunk.as_ref().len();
            self.chunk = Some(chunk);
        }

        self.get()
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
        assert_eq!(0, bytes.pos());
        assert_eq!(None, bytes.get());
    }

    #[test]
    fn bytes_next() {
        let mut pt = PieceTree::new();
        pt.insert_str(0, "foo");
        let mut bytes = pt.bytes();

        assert_eq!(0, bytes.pos());

        assert_eq!(as_byte("f"), bytes.get());
        assert_eq!(0, bytes.pos());

        assert_eq!(as_byte("o"), bytes.next());
        assert_eq!(1, bytes.pos());

        assert_eq!(as_byte("o"), bytes.next());
        assert_eq!(2, bytes.pos());

        assert!(bytes.next().is_none());
        assert!(bytes.get().is_none());
        assert_eq!(3, bytes.pos());
    }

    #[test]
    fn bytes_prev() {
        let mut pt = PieceTree::new();
        pt.insert_str(0, "foo");
        let mut bytes = pt.bytes_at(pt.len());

        assert!(bytes.get().is_none());
        assert_eq!(3, bytes.pos());
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
        assert_eq!(as_byte("f"), bytes.get());
    }

    #[test]
    fn bytes_back_and_forth() {
        let mut pt = PieceTree::new();
        pt.insert_str(0, "foo");
        let mut bytes = pt.bytes();

        assert_eq!(Some(b'f'), bytes.get());
        assert_eq!(0, bytes.pos());
        assert_eq!(as_byte("o"), bytes.next());
        assert_eq!(as_byte("o"), bytes.next());
        assert_eq!(None, bytes.next());
        assert_eq!(3, bytes.pos());
        assert_eq!(None, bytes.next());
        assert_eq!(3, bytes.pos());
        assert_eq!(None, bytes.next());
        assert_eq!(3, bytes.pos());
        assert!(bytes.get().is_none());
        assert_eq!(as_byte("o"), bytes.prev());
        assert_eq!(as_byte("o"), bytes.prev());
        assert_eq!(as_byte("f"), bytes.prev());
        assert_eq!(None, bytes.prev());
        assert_eq!(0, bytes.pos());
        assert_eq!(as_byte("f"), bytes.get());
    }

    #[test]
    fn bytes_start_middle() {
        let mut pt = PieceTree::new();
        pt.insert_str(0, "bar");
        pt.insert_str(0, "foo");
        let mut bytes = pt.bytes_at(3);

        assert_eq!(as_byte("b"), bytes.get());
        assert_eq!(3, bytes.pos());
        assert_eq!(as_byte("o"), bytes.prev());
        assert_eq!(2, bytes.pos());
        assert_eq!(as_byte("o"), bytes.prev());
        assert_eq!(1, bytes.pos());
        assert_eq!(as_byte("f"), bytes.prev());
        assert_eq!(0, bytes.pos());
    }
}
