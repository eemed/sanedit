use std::ops::Range;

use bstr::BString;

use super::{chunks::Chunks, Bytes, PieceTree};

#[derive(Debug, Clone)]
pub struct PieceTreeSlice<'a> {
    range: Range<usize>,
    pt: &'a PieceTree,
}

impl<'a> PieceTreeSlice<'a> {
    pub(crate) fn new(pt: &'a PieceTree, range: Range<usize>) -> PieceTreeSlice {
        PieceTreeSlice { range, pt }
    }

    #[inline]
    pub fn start(&self) -> usize {
        self.range.start
    }

    #[inline]
    pub fn end(&self) -> usize {
        self.range.end
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.range.len()
    }

    #[inline]
    pub fn bytes(&self) -> Bytes {
        self.bytes_at(0)
    }

    #[inline]
    pub fn bytes_at(&self, pos: usize) -> Bytes {
        debug_assert!(
            self.start() + pos <= self.pt.len,
            "bytes_at: Attempting to index {} over buffer len {}",
            self.start() + pos,
            self.pt.len
        );
        Bytes::new_from_slice(self.pt, pos, self.range.clone())
    }

    #[inline]
    pub fn chunks(&self) -> Chunks {
        self.chunks_at(0)
    }

    #[inline]
    pub fn chunks_at(&self, pos: usize) -> Chunks {
        debug_assert!(
            self.start() + pos <= self.pt.len,
            "chunks_at: Attempting to index {} over buffer len {}",
            self.start() + pos,
            self.pt.len
        );
        Chunks::new_from_slice(self.pt, pos, self.range.clone())
    }
}

impl<'a, B: AsRef<[u8]>> PartialEq<B> for PieceTreeSlice<'a> {
    fn eq(&self, other: &B) -> bool {
        if other.as_ref().len() != self.len() {
            return false;
        }

        let mut total = 0;
        let mut other = other.as_ref();
        let mut chunks = self.chunks();
        let mut pos_chunk = chunks.get();

        while let Some((_pos, chunk)) = pos_chunk {
            let chunk_bytes = chunk.as_ref();
            let chunk_len = chunk_bytes.len();

            if chunk_bytes != &other[..chunk_len] {
                return false;
            }
            other = &other[chunk_len..];

            total += chunk_len;

            pos_chunk = chunks.next();
        }

        total == self.len()
    }
}

impl<'a> From<&PieceTreeSlice<'a>> for Vec<u8> {
    fn from(slice: &PieceTreeSlice<'a>) -> Self {
        let mut bytes = Vec::with_capacity(slice.len());
        let mut chunks = slice.chunks();
        let mut pos_chunk = chunks.get();

        while let Some((_pos, chunk)) = pos_chunk {
            let chunk_bytes = chunk.as_ref();
            bytes.extend_from_slice(chunk_bytes);
            pos_chunk = chunks.next();
        }

        bytes
    }
}

impl<'a> From<&PieceTreeSlice<'a>> for String {
    fn from(slice: &PieceTreeSlice) -> Self {
        let bytes = Vec::from(slice);
        let byte_string = BString::from(bytes);
        byte_string.to_string()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn partial_eq() {
        let mut pt = PieceTree::new();
        pt.insert_str(0, "world");
        pt.insert_str(0, "hello ");

        let slice = pt.slice(3..9);
        let result = "lo wor";

        assert_eq!(result.to_string(), String::from(&slice));
        assert!(slice == result);
    }

    #[test]
    fn partial_eq_unbounded() {
        let mut pt = PieceTree::new();
        pt.insert_str(0, "world");
        pt.insert_str(0, "hello ");

        let slice = pt.slice(..);
        let result = "hello world";

        assert_eq!(result.to_string(), String::from(&slice));
        assert!(slice == result);
    }

    #[test]
    fn partial_ne() {
        let mut pt = PieceTree::new();
        pt.insert_str(0, "world");
        pt.insert_str(0, "hello ");

        let slice = pt.slice(6..);
        let result = " worl";

        assert_ne!(result.to_string(), String::from(slice.clone()));
        assert!(slice != result);
    }
}
