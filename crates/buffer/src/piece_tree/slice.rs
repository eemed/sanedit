use std::ops::{Bound, Range, RangeBounds};

use super::{
    chunks::Chunks,
    utf8::{self, chars::Chars, graphemes::Graphemes, lines::Lines},
    Bytes, PieceTreeView,
};

/// A read only slice of the piecetree
#[derive(Debug, Clone)]
pub struct PieceTreeSlice<'a> {
    pub(crate) range: Range<u64>,
    pub(crate) view: &'a PieceTreeView,
}

impl<'a> PieceTreeSlice<'a> {
    pub(crate) fn new(pt: &'a PieceTreeView, range: Range<u64>) -> PieceTreeSlice {
        PieceTreeSlice { range, view: pt }
    }

    /// Start of slice in buffer
    #[inline]
    pub fn start(&self) -> u64 {
        self.range.start
    }

    /// End of slice in buffer
    #[inline]
    pub fn end(&self) -> u64 {
        self.range.end
    }

    /// Range in buffer indices
    pub fn range(&self) -> Range<u64> {
        self.range.clone()
    }

    #[inline]
    pub fn len(&self) -> u64 {
        self.range.end - self.range.start
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.range.is_empty()
    }

    #[inline]
    pub fn bytes(&self) -> Bytes<'a> {
        self.bytes_at(0)
    }

    #[inline]
    pub fn bytes_at(&self, pos: u64) -> Bytes<'a> {
        debug_assert!(
            self.start() + pos <= self.view.len,
            "bytes_at: Attempting to index {} over buffer len {}",
            self.start() + pos,
            self.view.len
        );
        Bytes::new_from_slice(self, pos)
    }

    #[inline]
    pub fn chunks(&self) -> Chunks<'a> {
        self.chunks_at(0)
    }

    #[inline]
    pub fn chunks_at(&self, pos: u64) -> Chunks<'a> {
        debug_assert!(
            self.start() + pos <= self.view.len,
            "chunks_at: Attempting to index {} over buffer len {}",
            self.start() + pos,
            self.view.len
        );
        Chunks::new_from_slice(self, pos)
    }

    #[inline]
    pub fn chars(&self) -> Chars<'a> {
        self.chars_at(0)
    }

    #[inline]
    pub fn chars_at(&self, pos: u64) -> Chars<'a> {
        debug_assert!(
            self.start() + pos <= self.view.len,
            "chars_at: Attempting to index {} over buffer len {}",
            self.start() + pos,
            self.view.len
        );
        Chars::new_from_slice(self, pos)
    }

    #[inline]
    pub fn slice<R: RangeBounds<u64>>(&self, range: R) -> PieceTreeSlice<'a> {
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

        self.view.slice(start..end)
    }

    #[inline]
    pub fn lines(&self) -> Lines<'a> {
        self.lines_at(0)
    }

    #[inline]
    pub fn lines_at(&self, pos: u64) -> Lines<'a> {
        Lines::new_from_slice(self, pos)
    }

    /// Return the line number and slice at position
    #[inline]
    pub fn line_at(&self, pos: u64) -> (u64, PieceTreeSlice<'a>) {
        utf8::lines::line_at(self, pos)
    }

    /// Position at the start of line
    #[inline]
    pub fn pos_at_line(&self, line: u64) -> u64 {
        utf8::lines::pos_at_line(self, line)
    }

    #[inline]
    pub fn graphemes(&self) -> Graphemes<'a> {
        self.graphemes_at(0)
    }

    #[inline]
    pub fn graphemes_at(&self, pos: u64) -> Graphemes<'a> {
        Graphemes::new_from_slice(self, pos)
    }
}

impl<'a, B: AsRef<[u8]>> PartialEq<B> for PieceTreeSlice<'a> {
    fn eq(&self, other: &B) -> bool {
        if other.as_ref().len() as u64 != self.len() {
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
            total += chunk_len as u64;
            pos_chunk = chunks.next();
        }

        total == self.len()
    }
}

impl<'a> From<&PieceTreeSlice<'a>> for Vec<u8> {
    fn from(slice: &PieceTreeSlice<'a>) -> Self {
        assert!(
            slice.len() > usize::MAX as u64,
            "Slice is too large to be represented in memory"
        );

        let mut bytes = Vec::with_capacity(slice.len() as usize);
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
        let mut result = String::new();
        let mut chars = slice.chars();
        while let Some((_, _, ch)) = chars.next() {
            result.push(ch);
        }
        result
    }
}

#[cfg(test)]
mod test {
    use crate::PieceTree;

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
