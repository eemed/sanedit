use std::ops::{Bound, Range, RangeBounds};

use super::{chunks::Chunks, utf8::chars::Chars, Bytes, ReadOnlyPieceTree};

#[derive(Debug, Clone)]
pub struct PieceTreeSlice<'a> {
    range: Range<usize>,
    pt: &'a ReadOnlyPieceTree,
}

impl<'a> PieceTreeSlice<'a> {
    pub(crate) fn new(pt: &'a ReadOnlyPieceTree, range: Range<usize>) -> PieceTreeSlice {
        PieceTreeSlice { range, pt }
    }

    /// Start of slice in buffer
    #[inline]
    pub fn start(&self) -> usize {
        self.range.start
    }

    /// End of slice in buffer
    #[inline]
    pub fn end(&self) -> usize {
        self.range.end
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.range.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.range.is_empty()
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

    #[inline]
    pub fn chars(&self) -> Chars {
        self.chars_at(0)
    }

    #[inline]
    pub fn chars_at(&self, pos: usize) -> Chars {
        debug_assert!(
            self.start() + pos <= self.pt.len,
            "chars_at: Attempting to index {} over buffer len {}",
            self.start() + pos,
            self.pt.len
        );
        Chars::new_from_slice(self.pt, pos, self.range.clone())
    }

    #[inline]
    pub fn slice<R: RangeBounds<usize>>(&self, range: R) -> PieceTreeSlice<'a> {
        let sub_start = match range.start_bound() {
            Bound::Included(n) => *n,
            Bound::Excluded(n) => *n + 1,
            Bound::Unbounded => 0,
        };

        let sub_end = match range.end_bound() {
            Bound::Included(n) => *n + 1,
            Bound::Excluded(n) => *n,
            Bound::Unbounded => self.len(),
        };

        let start = self.range.start + sub_start;
        let end = self.range.start + sub_end;

        self.pt.slice(start..end)
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn partial_eq() {
        let mut pt = PieceTree::new();
        pt.insert(0, "world");
        pt.insert(0, "hello ");

        let slice = pt.slice(3..9);
        let result = "lo wor";

        assert_eq!(result.to_string(), String::from(&slice));
        assert!(slice == result);
    }

    #[test]
    fn partial_eq_unbounded() {
        let mut pt = PieceTree::new();
        pt.insert(0, "world");
        pt.insert(0, "hello ");

        let slice = pt.slice(..);
        let result = "hello world";

        assert_eq!(result.to_string(), String::from(&slice));
        assert!(slice == result);
    }

    #[test]
    fn partial_ne() {
        let mut pt = PieceTree::new();
        pt.insert(0, "world");
        pt.insert(0, "hello ");

        let slice = pt.slice(6..);
        let result = " worl";

        assert_ne!(result.to_string(), String::from(&slice));
        assert!(slice != result);
    }
}
