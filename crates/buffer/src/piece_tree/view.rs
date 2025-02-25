use std::cmp;
use std::io::{self, Write};
use std::ops::{Bound, RangeBounds};
use std::sync::Arc;

use super::buffers::AddBufferReader;
use super::buffers::OriginalBuffer;
use super::inplace::write_in_place;
use super::mark::Mark;
use super::tree::pieces::Pieces;
use super::tree::Tree;
use super::utf8::graphemes::Graphemes;
use crate::piece_tree::buffers::BufferKind;
use crate::piece_tree::chunks::Chunks;
use crate::piece_tree::utf8::lines::Lines;

use super::slice::PieceTreeSlice;
use super::utf8::chars::Chars;
use crate::piece_tree::bytes::Bytes;

/// A read only view of the piece tree
///
/// Similar to a slice(..), but owning.
/// The underlying pointers are cloned instead of referencing the PieceTree.
#[derive(Clone, Debug)]
pub struct PieceTreeView {
    pub(crate) orig: Arc<OriginalBuffer>,
    pub(crate) add: AddBufferReader,
    pub(crate) tree: Tree,
    pub(crate) len: u64,
}

impl PieceTreeView {
    #[inline]
    pub fn bytes(&self) -> Bytes {
        self.bytes_at(0)
    }

    #[inline]
    pub fn bytes_at(&self, pos: u64) -> Bytes {
        debug_assert!(
            pos <= self.len,
            "bytes_at: Attempting to index {} over buffer len {}",
            pos,
            self.len
        );
        Bytes::new(self, pos)
    }

    #[inline]
    pub fn chunks(&self) -> Chunks {
        self.chunks_at(0)
    }

    #[inline]
    pub fn chunks_at(&self, pos: u64) -> Chunks {
        debug_assert!(
            pos <= self.len,
            "chunks_at: Attempting to index {} over buffer len {}",
            pos,
            self.len
        );
        Chunks::new(self, pos)
    }

    #[inline]
    pub fn slice<R: RangeBounds<u64>>(&self, range: R) -> PieceTreeSlice {
        let start = match range.start_bound() {
            Bound::Included(n) => *n,
            Bound::Excluded(n) => *n + 1,
            Bound::Unbounded => 0,
        };

        let end = match range.end_bound() {
            Bound::Included(n) => *n + 1,
            Bound::Excluded(n) => *n,
            Bound::Unbounded => self.len,
        };

        PieceTreeSlice::new(self, start..end)
    }

    #[inline]
    pub fn chars(&self) -> Chars {
        self.chars_at(0)
    }

    #[inline]
    pub fn chars_at(&self, at: u64) -> Chars {
        Chars::new(self, at)
    }

    #[inline]
    pub fn lines(&self) -> Lines {
        self.lines_at(0)
    }

    #[inline]
    pub fn lines_at(&self, pos: u64) -> Lines {
        debug_assert!(
            pos <= self.len,
            "lines_at: Attempting to index {} over buffer len {}",
            pos,
            self.len
        );
        Lines::new(self, pos)
    }

    #[inline]
    pub fn line_at(&self, pos: u64) -> (u64, PieceTreeSlice) {
        debug_assert!(
            pos <= self.len,
            "line_at: Attempting to index {} over buffer len {}",
            pos,
            self.len
        );
        self.slice(..).line_at(pos)
    }

    #[inline]
    pub fn pos_at_line(&self, line: u64) -> u64 {
        self.slice(..).pos_at_line(line)
    }

    #[inline]
    pub fn graphemes(&self) -> Graphemes {
        self.graphemes_at(0)
    }

    #[inline]
    pub fn graphemes_at(&self, pos: u64) -> Graphemes {
        debug_assert!(
            pos <= self.len,
            "graphemes_at: Attempting to index {} over buffer len {}",
            pos,
            self.len
        );
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
            pos <= self.len,
            "mark: Attempting to index {} over buffer len {}",
            pos,
            self.len
        );
        let after = pos == self.len();
        if after {
            // If marking an empty buffer use original 0 and after flag
            if pos == 0 {
                return Mark {
                    orig: 0,
                    kind: BufferKind::Original,
                    pos: 0,
                    count: 0,
                    after,
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
            orig: pos,
            kind: piece.kind,
            pos: piece.pos + off,
            count: piece.count,
            after,
        }
    }

    /// Get a buffer position from a mark.
    /// If the buffer position has been deleted returns the original mark
    /// position.
    #[inline]
    pub fn mark_to_pos(&self, mark: &Mark) -> u64 {
        // Marked an empty buffer
        if mark.orig == 0 && mark.after {
            return 0;
        }

        let mut pieces = Pieces::new(self, 0);
        let mut piece = pieces.get();

        while let Some((p_pos, p)) = piece {
            if p.kind == mark.kind
                && p.pos <= mark.pos
                && mark.pos < p.pos + p.len
                && mark.count == p.count
            {
                let mut off = mark.pos - p.pos;
                if mark.after {
                    off += 1;
                }
                return p_pos + off;
            }
            piece = pieces.next();
        }

        cmp::min(mark.orig, self.len)
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
    pub unsafe fn write_in_place(&self) -> io::Result<()> {
        write_in_place(self)
    }

    pub fn write_to<W: Write>(&self, mut writer: W) -> io::Result<usize> {
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

    #[inline]
    pub fn len(&self) -> u64 {
        self.len
    }

    #[inline]
    pub fn piece_count(&self) -> usize {
        self.tree.node_count
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl From<&PieceTreeView> for Vec<u8> {
    fn from(pt: &PieceTreeView) -> Self {
        let slice = pt.slice(..);
        (&slice).into()
    }
}

impl From<&PieceTreeView> for String {
    fn from(value: &PieceTreeView) -> Self {
        let slice = value.slice(..);
        String::from(&slice)
    }
}
