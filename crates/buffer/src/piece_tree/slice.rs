use std::{
    io,
    ops::{Bound, Range, RangeBounds},
    sync::Arc,
};

use super::{
    buffers::{AddBufferReader, OriginalBuffer},
    chunks::Chunks,
    inplace::write_in_place,
    tree::Tree,
    utf8::{self, chars::Chars, graphemes::Graphemes, lines::Lines},
    Bytes,
};
use crate::{
    piece_tree::{buffers::BufferKind, tree::pieces::Pieces},
    Mark, MarkResult,
};

/// A read only slice of the piecetree
#[derive(Debug, Clone)]
pub struct PieceTreeSlice {
    pub(crate) range: Range<u64>,
    pub(crate) orig: Arc<OriginalBuffer>,
    pub(crate) add: AddBufferReader,
    pub(crate) tree: Tree,
}

impl PieceTreeSlice {
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
    pub fn bytes(&self) -> Bytes<'_> {
        self.bytes_at(0)
    }

    #[inline]
    pub fn bytes_at(&self, pos: u64) -> Bytes<'_> {
        debug_assert!(
            pos <= self.len(),
            "bytes_at: Attempting to index {} over slice len {}",
            pos,
            self.len(),
        );
        Bytes::new(self, pos)
    }

    #[inline]
    pub fn chunks(&self) -> Chunks<'_> {
        self.chunks_at(0)
    }

    #[inline]
    pub fn chunks_at(&self, pos: u64) -> Chunks<'_> {
        debug_assert!(
            pos <= self.len(),
            "chunks_at: Attempting to index {} over slice len {}",
            pos,
            self.len()
        );
        Chunks::new(self, pos)
    }

    #[inline]
    pub fn chars(&self) -> Chars<'_> {
        self.chars_at(0)
    }

    #[inline]
    pub fn chars_at(&self, pos: u64) -> Chars<'_> {
        debug_assert!(
            pos <= self.len(),
            "chars_at: Attempting to index {} over slice len {}",
            pos,
            self.len()
        );
        Chars::new(self, pos)
    }

    #[inline]
    pub fn slice<R: RangeBounds<u64>>(&self, range: R) -> PieceTreeSlice {
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

        debug_assert!(self.start() <= start, "Slicing over original slice start");
        debug_assert!(end <= self.end(), "Slicing over original slice end");

        PieceTreeSlice {
            range: start..end,
            orig: self.orig.clone(),
            add: self.add.clone(),
            tree: self.tree.clone(),
        }
    }

    #[inline]
    pub fn lines(&self) -> Lines<'_> {
        self.lines_at(0)
    }

    #[inline]
    pub fn lines_at(&self, pos: u64) -> Lines<'_> {
        Lines::new(self, pos)
    }

    /// Return the line number and slice at position
    #[inline]
    pub fn line_at(&self, pos: u64) -> (u64, PieceTreeSlice) {
        utf8::lines::line_at(self, pos)
    }

    /// Position at the start of line
    #[inline]
    pub fn pos_at_line(&self, line: u64) -> Option<u64> {
        utf8::lines::pos_at_line(self, line)
    }

    #[inline]
    pub fn graphemes(&self) -> Graphemes<'_> {
        self.graphemes_at(0)
    }

    #[inline]
    pub fn graphemes_at(&self, pos: u64) -> Graphemes<'_> {
        Graphemes::new(self, pos)
    }

    #[inline]
    pub fn is_file_backed(&self) -> bool {
        self.orig.is_file_backed()
    }

    /// Mark a position in the buffer
    // Internally works using offsets into the read only and append only buffers.
    // These can be safely indexed into because they never change after written.
    // Searching for a mark is O(n) operation where n is the number of pieces in the
    // piece tree
    #[inline]
    pub fn mark(&self, mut pos: u64) -> Mark {
        debug_assert!(
            pos <= self.len(),
            "mark: Attempting to index {} over slice len {}",
            pos,
            self.len()
        );
        let end_of_buffer = pos == self.len();
        if end_of_buffer {
            // If marking an empty buffer use original 0 and after flag
            if pos == 0 {
                return Mark {
                    orig: 0,
                    kind: BufferKind::Original,
                    pos: 0,
                    count: 0,
                    end_of_buffer,
                };
            }

            pos -= 1;
        }

        let pieces = Pieces::new(self, pos);
        let (p_pos, piece) = pieces
            .get()
            .unwrap_or_else(|| panic!("Cannot find a piece for position {}", pos));
        let off = pos - p_pos;
        Mark {
            orig: self.start() + pos,
            kind: piece.kind,
            pos: piece.pos + off,
            count: piece.count,
            end_of_buffer,
        }
    }

    /// Get a buffer position from a mark.
    /// If the buffer position has been deleted returns the original mark
    /// position.
    #[inline]
    pub fn mark_to_pos(&self, mark: &Mark) -> MarkResult {
        // Marked an empty buffer
        if mark.orig == 0 && mark.end_of_buffer {
            return MarkResult::Found(0);
        }

        let mut min_dist = self.len();
        let mut deleted_closest = std::cmp::min(mark.orig, self.len());
        let mut pieces = Pieces::new(self, 0);
        let mut piece = pieces.get();

        while let Some((p_pos, p)) = piece {
            if p.kind == mark.kind && mark.count == p.count {
                // If mark in piece we found it
                if p.pos <= mark.pos && mark.pos < p.pos + p.len {
                    let mut off = mark.pos - p.pos;
                    if mark.end_of_buffer {
                        off += 1;
                    }
                    return MarkResult::Found(p_pos + off);
                }

                // Try to find closest match if mark position is deleted
                let dist = p.min_abs_distance(mark.pos);
                if dist < min_dist {
                    min_dist = dist;
                    deleted_closest = mark.pos + if mark.end_of_buffer { 1 } else { 0 };
                }
            }

            piece = pieces.next();
        }

        MarkResult::Deleted(deleted_closest)
    }

    ///
    /// Writes the file in place if the buffer is file backed
    ///
    /// UNSAFETY: All previously created ReadOnlyPieceTrees cannot be used
    /// anymore.
    ///
    /// Good:
    ///      1. If only replaced or appended bytes, saving will be very fast
    ///      2. No need for additional disk space as no copy is created
    ///
    /// Bad:
    ///      1. If io error occurs while saving the file will be left in an
    ///         incomplete state
    ///      2. Probably slower than writing a copy if insert/remove operations are
    ///         in the beginning portion of the file
    ///      3. Previously created read only copies/marks cannot be used anymore
    pub(crate) unsafe fn write_in_place(&self) -> io::Result<()> {
        write_in_place(self)
    }

    pub fn write_to<W: io::Write>(&self, mut writer: W) -> io::Result<usize> {
        let mut written = 0;
        let mut chunks = self.chunks();
        let mut pos_chunk = chunks.get();

        while let Some((_pos, chunk)) = pos_chunk {
            let chunk_bytes = chunk.as_ref();
            writer.write_all(chunk_bytes)?;
            written += chunk_bytes.len();
            pos_chunk = chunks.next();
        }

        writer.flush()?;
        Ok(written)
    }
}

impl<B: AsRef<[u8]>> PartialEq<B> for PieceTreeSlice {
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

impl From<&PieceTreeSlice> for Vec<u8> {
    fn from(slice: &PieceTreeSlice) -> Self {
        assert!(
            slice.len() < usize::MAX as u64,
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

impl From<&PieceTreeSlice> for String {
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
