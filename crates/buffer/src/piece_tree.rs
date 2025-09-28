pub(crate) mod buffers;
pub(crate) mod builder;
pub(crate) mod bytes;
pub(crate) mod chunks;
pub(crate) mod inplace;
pub(crate) mod mark;
pub(crate) mod slice;
pub(crate) mod tree;
pub(crate) mod utf8;
pub(crate) mod view;

use std::borrow::Cow;
use std::io::{self, Write};
use std::ops::RangeBounds;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use self::buffers::{AddBuffer, AddBufferWriter};
use self::mark::Mark;
use self::tree::Tree;
use self::utf8::graphemes::Graphemes;
use self::view::PieceTreeView;
use crate::piece_tree::buffers::{AppendResult, BufferKind};
use crate::piece_tree::chunks::Chunks;
use crate::piece_tree::tree::piece::Piece;
use crate::piece_tree::utf8::lines::Lines;
use buffers::OriginalBuffer;
use mark::MarkResult;

use self::slice::PieceTreeSlice;
use self::utf8::chars::Chars;
use crate::piece_tree::bytes::Bytes;

pub(crate) const FILE_BACKED_MAX_PIECE_SIZE: usize = 1024 * 256;

/// Buffer is created using two buffers. Original buffer which stores original
/// file content and is immutable and add buffer which stores added text and is
/// append only. Then pieces referencing parts of these two buffers are in a
/// red-black tree datastructure. The buffer contents can be now constructed by
/// traversing the tree in-order and getting the buffer parts that the pieces
/// reference.
///
/// The tree implementation uses copy-on-write. This means we can easily create
/// snapshots from the tree. These copies are relatively lightweight as the
/// tree data structure can be shared among copies. Data is only copied when
/// modifying the tree and still holding snaphots.
#[derive(Debug)]
pub struct PieceTree {
    add_writer: AddBufferWriter,
    view: PieceTreeView,
}

impl PieceTree {
    /// Create a new empty piece tree
    #[inline]
    pub fn new() -> PieceTree {
        let orig_buf = OriginalBuffer::new();
        Self::from_original_buffer(orig_buf)
    }

    /// Create a new piece tree from a reader.
    /// The content is stored in memory.
    #[inline]
    pub fn from_reader<R: io::Read>(reader: R) -> io::Result<PieceTree> {
        let orig_buf = OriginalBuffer::from_reader(reader)?;
        Ok(Self::from_original_buffer(orig_buf))
    }

    /// Create a file backed buffer
    #[inline]
    pub fn from_path<P: AsRef<Path>>(path: P) -> io::Result<PieceTree> {
        let orig_buf = OriginalBuffer::from_path(path)?;
        Ok(Self::from_original_buffer(orig_buf))
    }

    #[inline]
    fn from_original_buffer(orig: OriginalBuffer) -> PieceTree {
        let orig = Arc::new(orig);
        let (aread, awrite) = AddBuffer::split();
        let mut pieces = Tree::new();

        if !orig.is_empty() {
            if orig.is_file_backed() {
                // Split into multiple pieces if file backed.
                // This prevents reading very large chunks into memory.
                let mut pos = 0;
                let mut len = orig.len();
                while len != 0 {
                    let plen = len.min(FILE_BACKED_MAX_PIECE_SIZE as u64);
                    let piece = Piece::new(BufferKind::Original, pos, plen);
                    pieces.insert(pos, piece, true);

                    len -= plen;
                    pos += plen;
                }
            } else {
                let piece = Piece::new(BufferKind::Original, 0, orig.len());
                pieces.insert(0, piece, true);
            }
        }

        PieceTree {
            add_writer: awrite,
            view: PieceTreeView {
                tree: pieces,
                len: orig.len(),
                orig,
                add: aread,
            },
        }
    }

    /// Insert the same bytes to multiple places at once
    /// This helps with piece tree fragmentation, if the same bytes are inserted
    /// multiple times.
    ///
    /// The bytes are appended to add buffer once and just referenced multiple
    /// times. This allows for example multiple cursors to append to existing
    /// pieces if insertions are sequential.
    ///
    /// If multiple cursors inserted bytes using insert() instead, each
    /// insertion would create a new piece because the content in add buffer
    /// would not be sequential. Creating M x N pieces where M is the number of
    /// cursors and N is the number of edits characters.
    pub fn insert_multi<B: AsRef<[u8]>>(&mut self, positions: &[u64], bytes: B) {
        let mut bytes = bytes.as_ref();
        if bytes.is_empty() {
            return;
        }

        let positions: Cow<[u64]> = if is_sorted(positions) {
            positions.into()
        } else {
            let mut poss: Vec<u64> = positions.into();
            poss.sort();
            poss.into()
        };

        let mut inserted = 0;
        while !bytes.is_empty() {
            let bpos = self.add_writer.len();
            let (n, can_append) = match self.add_writer.append_slice(bytes) {
                AppendResult::NewBlock(n) => (n, false),
                AppendResult::Append(n) => (n, true),
            };

            for (count, pos) in positions.iter().enumerate() {
                let piece =
                    Piece::new_with_count(BufferKind::Add, bpos as u64, n as u64, count as u32);
                self.view.len += piece.len;
                let inserted_now =
                    (inserted * (count as u64 + 1)) + (n as u64 * count as u64);
                self.view
                    .tree
                    .insert(*pos + inserted_now, piece, can_append);
            }

            inserted += n as u64;
            bytes = &bytes[n..];
        }
    }

    /// Insert bytes to a position
    pub fn insert<B: AsRef<[u8]>>(&mut self, mut pos: u64, bytes: B) {
        debug_assert!(
            pos <= self.view.len,
            "insert: Attempting to index {} over buffer len {}",
            pos,
            self.view.len
        );

        let mut bytes = bytes.as_ref();
        if bytes.is_empty() {
            return;
        }

        while !bytes.is_empty() {
            let bpos = self.add_writer.len();
            let (n, can_append) = match self.add_writer.append_slice(bytes) {
                AppendResult::NewBlock(n) => (n, false),
                AppendResult::Append(n) => (n, true),
            };

            let piece = Piece::new(BufferKind::Add, bpos as u64, n as u64);
            self.view.len += piece.len;
            self.view.tree.insert(pos, piece, can_append);

            pos += n as u64;
            bytes = &bytes[n..];
        }
    }

    #[inline]
    pub fn insert_char(&mut self, pos: u64, ch: char) {
        let mut buf = [0; 4];
        let string = ch.encode_utf8(&mut buf);
        self.insert(pos, string);
    }

    #[inline]
    pub fn remove<R: RangeBounds<u64>>(&mut self, range: R) {
        let start = match range.start_bound() {
            std::ops::Bound::Included(n) => *n,
            std::ops::Bound::Excluded(n) => *n + 1,
            std::ops::Bound::Unbounded => 0,
        };

        let end = match range.end_bound() {
            std::ops::Bound::Included(n) => *n + 1,
            std::ops::Bound::Excluded(n) => *n,
            std::ops::Bound::Unbounded => self.view.len,
        };

        debug_assert!(
            end <= self.view.len,
            "remove: Attempting to index {} over buffer len {}",
            end,
            self.view.len
        );

        self.view.tree.remove(start..end);
        self.view.len -= end - start;
    }

    #[inline]
    pub fn append<B: AsRef<[u8]>>(&mut self, bytes: B) {
        self.insert(self.view.len, bytes);
    }

    #[inline]
    pub fn bytes(&self) -> Bytes<'_> {
        self.view.bytes()
    }

    #[inline]
    pub fn bytes_at(&self, pos: u64) -> Bytes<'_> {
        self.view.bytes_at(pos)
    }

    #[inline]
    pub fn chunks(&self) -> Chunks<'_> {
        self.view.chunks()
    }

    #[inline]
    pub fn chunks_at(&self, pos: u64) -> Chunks<'_> {
        self.view.chunks_at(pos)
    }

    #[inline]
    pub fn slice<R: RangeBounds<u64>>(&self, range: R) -> PieceTreeSlice<'_> {
        self.view.slice(range)
    }

    #[inline]
    pub fn chars(&self) -> Chars<'_> {
        self.view.chars()
    }

    #[inline]
    pub fn chars_at(&self, at: u64) -> Chars<'_> {
        self.view.chars_at(at)
    }

    #[inline]
    pub fn lines(&self) -> Lines<'_> {
        self.view.lines()
    }

    #[inline]
    pub fn lines_at(&self, pos: u64) -> Lines<'_> {
        self.view.lines_at(pos)
    }

    #[inline]
    pub fn line_at(&self, pos: u64) -> (u64, PieceTreeSlice<'_>) {
        self.view.line_at(pos)
    }

    #[inline]
    pub fn pos_at_line(&self, line: u64) -> Option<u64> {
        self.view.pos_at_line(line)
    }

    #[inline]
    pub fn graphemes(&self) -> Graphemes<'_> {
        self.view.graphemes()
    }

    #[inline]
    pub fn graphemes_at(&self, pos: u64) -> Graphemes<'_> {
        self.view.graphemes_at(pos)
    }

    #[inline]
    pub fn is_file_backed(&self) -> bool {
        self.view.is_file_backed()
    }

    /// Mark a position in the buffer
    // Internally works using offsets into the read only and append only buffers.
    // These can be safely indexed into because they never change after written.
    // Searching for a mark is O(n) operation where n is the number of pieces in the
    // piece tree
    #[inline]
    pub fn mark(&self, pos: u64) -> Mark {
        self.view.mark(pos)
    }

    /// Get a buffer position from a mark.
    /// If the buffer position has been deleted returns the original mark
    /// position.
    #[inline]
    pub fn mark_to_pos(&self, mark: &Mark) -> MarkResult {
        self.view.mark_to_pos(mark)
    }

    #[inline]
    pub fn write_to<W: Write>(&self, writer: W) -> io::Result<usize> {
        self.view.write_to(writer)
    }

    /// If the buffer is file backed, renames the backing file to another one.
    #[inline]
    pub fn rename_backing_file<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        self.view.orig.rename_backing_file(path)
    }

    #[inline]
    pub fn backing_file(&self) -> Option<PathBuf> {
        self.view.orig.file_path()
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
    #[allow(dead_code)]
    unsafe fn write_in_place(&self) -> io::Result<()> {
        self.view.write_in_place()
    }

    #[inline]
    pub fn len(&self) -> u64 {
        self.view.len()
    }

    #[inline]
    pub fn piece_count(&self) -> usize {
        self.view.piece_count()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.view.is_empty()
    }

    #[inline]
    pub fn view(&self) -> PieceTreeView {
        self.view.clone()
    }

    #[inline]
    pub fn restore(&mut self, ro: PieceTreeView) {
        self.view = ro;
    }
}

impl<A: AsRef<[u8]>> From<A> for PieceTree {
    fn from(value: A) -> Self {
        PieceTree::from_reader(io::Cursor::new(value.as_ref())).unwrap()
    }
}

impl From<&PieceTree> for String {
    fn from(value: &PieceTree) -> Self {
        let slice = value.slice(..);
        String::from(&slice)
    }
}

impl From<&PieceTree> for Vec<u8> {
    fn from(pt: &PieceTree) -> Self {
        let view = &pt.view;
        view.into()
    }
}

impl Default for PieceTree {
    fn default() -> Self {
        PieceTree::new()
    }
}

fn is_sorted(arr: &[u64]) -> bool {
    let mut min = 0;

    for item in arr {
        if min > *item {
            return false;
        }

        min = *item;
    }

    true
}

// #[cfg(test)]
// pub(crate) mod test {
//     use super::*;

//     #[test]
//     fn crash_test() {
//         let mut pt = PieceTree::new();

//         loop {
//             println!("Insert: {}", pt.piece_count());
//             pt.insert(0, b"a");
//         }
//     }
// }
