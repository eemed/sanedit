pub(crate) mod buffers;
pub(crate) mod builder;
pub(crate) mod bytes;
pub(crate) mod chunks;
pub(crate) mod inplace;
pub(crate) mod positions;
pub(crate) mod slice;
pub(crate) mod tree;
pub(crate) mod utf8;

use std::cmp;
use std::io::{self, Write};
use std::ops::{Bound, RangeBounds};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use self::buffers::{AddBuffer, AddBufferReader, AddBufferWriter};
use self::tree::pieces::Pieces;
use self::tree::Tree;
use self::utf8::graphemes::Graphemes;
use crate::piece_tree::buffers::{AppendResult, BufferKind};
use crate::piece_tree::chunks::Chunks;
use crate::piece_tree::tree::piece::Piece;
use crate::piece_tree::utf8::lines::Lines;
use buffers::OriginalBuffer;

use self::slice::PieceTreeSlice;
use self::utf8::chars::Chars;
use crate::piece_tree::bytes::Bytes;
pub use positions::SortedPositions;

pub(crate) const FILE_BACKED_MAX_PIECE_SIZE: usize = 1024 * 256;

/// A mark that tracks a position in text.
/// It can be retrieved anytime if the position has not been deleted
#[derive(Debug, Clone, Copy)]
pub struct Mark {
    pub(crate) orig: usize,
    pub(crate) kind: BufferKind,
    pub(crate) pos: usize,
    pub(crate) count: u32,
}

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
    pt: ReadOnlyPieceTree,
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

    #[inline]
    pub fn from_path<P: AsRef<Path>>(path: P) -> io::Result<PieceTree> {
        let orig_buf = OriginalBuffer::from_path(path)?;
        Ok(Self::from_original_buffer(orig_buf))
    }

    #[inline]
    fn from_original_buffer(orig: OriginalBuffer) -> PieceTree {
        let orig = Arc::new(orig);
        let (aread, awrite) = AddBuffer::new();
        let mut pieces = Tree::new();

        if !orig.is_empty() {
            if orig.is_file_backed() {
                // Split into multiple pieces if file backed.
                // This prevents reading very large chunks into memory.
                let mut pos = 0;
                let mut len = orig.len();
                while len != 0 {
                    let plen = len.min(FILE_BACKED_MAX_PIECE_SIZE);
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
            pt: ReadOnlyPieceTree {
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
    pub fn insert_multi<B: AsRef<[u8]>>(&mut self, positions: &SortedPositions, bytes: B) {
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

            for (count, pos) in positions.iter().rev().enumerate() {
                let piece = Piece::new_with_count(BufferKind::Add, bpos, bytes.len(), count as u32);
                self.pt.len += piece.len;
                self.pt.tree.insert(*pos, piece, can_append);
            }

            bytes = &bytes[n..];
        }
    }

    /// Insert bytes to a position
    pub fn insert<B: AsRef<[u8]>>(&mut self, pos: usize, bytes: B) {
        debug_assert!(
            pos <= self.pt.len,
            "insert: Attempting to index {} over buffer len {}",
            pos,
            self.pt.len
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

            let piece = Piece::new(BufferKind::Add, bpos, bytes.len());
            self.pt.len += piece.len;
            self.pt.tree.insert(pos, piece, can_append);

            bytes = &bytes[n..];
        }
    }

    #[inline]
    pub fn insert_char(&mut self, pos: usize, ch: char) {
        let mut buf = [0; 4];
        let string = ch.encode_utf8(&mut buf);
        self.insert(pos, string);
    }

    #[inline]
    pub fn remove<R: RangeBounds<usize>>(&mut self, range: R) {
        let start = match range.start_bound() {
            std::ops::Bound::Included(n) => *n,
            std::ops::Bound::Excluded(n) => *n + 1,
            std::ops::Bound::Unbounded => 0,
        };

        let end = match range.end_bound() {
            std::ops::Bound::Included(n) => *n + 1,
            std::ops::Bound::Excluded(n) => *n,
            std::ops::Bound::Unbounded => self.pt.len,
        };

        debug_assert!(
            end <= self.pt.len,
            "remove: Attempting to index {} over buffer len {}",
            end,
            self.pt.len
        );

        self.pt.tree.remove(start..end);
        self.pt.len -= end - start;
    }

    #[inline]
    pub fn append<B: AsRef<[u8]>>(&mut self, bytes: B) {
        self.insert(self.pt.len, bytes);
    }

    #[inline]
    pub fn bytes(&self) -> Bytes {
        self.pt.bytes()
    }

    #[inline]
    pub fn bytes_at(&self, pos: usize) -> Bytes {
        self.pt.bytes_at(pos)
    }

    #[inline]
    pub fn chunks(&self) -> Chunks {
        self.pt.chunks()
    }

    #[inline]
    pub fn chunks_at(&self, pos: usize) -> Chunks {
        self.pt.chunks_at(pos)
    }

    #[inline]
    pub fn slice<R: RangeBounds<usize>>(&self, range: R) -> PieceTreeSlice {
        self.pt.slice(range)
    }

    #[inline]
    pub fn chars(&self) -> Chars {
        self.pt.chars()
    }

    #[inline]
    pub fn chars_at(&self, at: usize) -> Chars {
        self.pt.chars_at(at)
    }

    #[inline]
    pub fn lines(&self) -> Lines {
        self.pt.lines()
    }

    #[inline]
    pub fn lines_at(&self, pos: usize) -> Lines {
        self.pt.lines_at(pos)
    }

    #[inline]
    pub fn line_at(&self, pos: usize) -> usize {
        self.pt.line_at(pos)
    }

    #[inline]
    pub fn graphemes(&self) -> Graphemes {
        self.pt.graphemes()
    }

    #[inline]
    pub fn graphemes_at(&self, pos: usize) -> Graphemes {
        self.pt.graphemes_at(pos)
    }

    #[inline]
    pub fn is_file_backed(&self) -> bool {
        self.pt.is_file_backed()
    }

    /// Mark a position in the buffer
    // Internally works using offsets into the read only and append only buffers.
    // These can be safely indexed into because they never change after written.
    // Searching for a mark is O(n) operation where n is the number of pieces in the
    // piece tree
    #[inline]
    pub fn mark(&self, pos: usize) -> Mark {
        self.pt.mark(pos)
    }

    /// Get a buffer position from a mark.
    /// If the buffer position has been deleted returns the original mark
    /// position.
    #[inline]
    pub fn mark_to_pos(&self, mark: &Mark) -> usize {
        self.pt.mark_to_pos(mark)
    }

    #[inline]
    pub fn write_to<W: Write>(&self, writer: W) -> io::Result<usize> {
        self.pt.write_to(writer)
    }

    /// If the buffer is file backed, renames the backing file to another one.
    #[inline]
    pub fn rename_backing_file<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        self.pt.orig.rename_backing_file(path)
    }

    #[inline]
    pub fn backing_file(&self) -> Option<PathBuf> {
        self.pt.orig.file_path()
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
    ///      3. Previously created undo points/marks cannot be used anymore
    unsafe fn write_in_place(self) -> io::Result<()> {
        inplace::write_in_place(&self.pt)
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.pt.len()
    }

    #[inline]
    pub fn piece_count(&self) -> usize {
        self.pt.piece_count()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.pt.is_empty()
    }

    #[inline]
    pub fn read_only_copy(&self) -> ReadOnlyPieceTree {
        self.pt.clone()
    }

    #[inline]
    pub fn restore(&mut self, ro: ReadOnlyPieceTree) {
        self.pt = ro;
    }

    #[inline]
    pub(crate) fn tree(&self) -> &Tree {
        &self.pt.tree
    }
}

impl Default for PieceTree {
    fn default() -> Self {
        PieceTree::new()
    }
}

#[derive(Clone, Debug)]
pub struct ReadOnlyPieceTree {
    pub(crate) orig: Arc<OriginalBuffer>,
    pub(crate) add: AddBufferReader,
    pub(crate) tree: Tree,
    pub(crate) len: usize,
}

impl ReadOnlyPieceTree {
    #[inline]
    pub fn bytes(&self) -> Bytes {
        self.bytes_at(0)
    }

    #[inline]
    pub fn bytes_at(&self, pos: usize) -> Bytes {
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
    pub fn chunks_at(&self, pos: usize) -> Chunks {
        debug_assert!(
            pos <= self.len,
            "chunks_at: Attempting to index {} over buffer len {}",
            pos,
            self.len
        );
        Chunks::new(self, pos)
    }

    #[inline]
    pub fn slice<R: RangeBounds<usize>>(&self, range: R) -> PieceTreeSlice {
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
    pub fn chars_at(&self, at: usize) -> Chars {
        Chars::new(self, at)
    }

    #[inline]
    pub fn lines(&self) -> Lines {
        self.lines_at(0)
    }

    #[inline]
    pub fn lines_at(&self, pos: usize) -> Lines {
        debug_assert!(
            pos <= self.len,
            "lines_at: Attempting to index {} over buffer len {}",
            pos,
            self.len
        );
        Lines::new(self, pos)
    }

    #[inline]
    pub fn line_at(&self, pos: usize) -> usize {
        debug_assert!(
            pos <= self.len,
            "line_at: Attempting to index {} over buffer len {}",
            pos,
            self.len
        );
        self.slice(..).line_at(pos)
    }

    #[inline]
    pub fn graphemes(&self) -> Graphemes {
        self.graphemes_at(0)
    }

    #[inline]
    pub fn graphemes_at(&self, pos: usize) -> Graphemes {
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
    pub fn mark(&self, pos: usize) -> Mark {
        debug_assert!(
            pos <= self.len,
            "mark: Attempting to index {} over buffer len {}",
            pos,
            self.len
        );
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
        }
    }

    /// Get a buffer position from a mark.
    /// If the buffer position has been deleted returns the original mark
    /// position.
    #[inline]
    pub fn mark_to_pos(&self, mark: &Mark) -> usize {
        let mut pieces = Pieces::new(self, 0);
        let mut piece = pieces.get();

        while let Some((p_pos, p)) = piece {
            if p.kind == mark.kind
                && p.pos <= mark.pos
                && mark.pos < p.pos + p.len
                && mark.count == p.count
            {
                let off = mark.pos - p.pos;
                return p_pos + off;
            }
            piece = pieces.next();
        }

        cmp::min(mark.orig, self.len)
    }

    pub fn write_in_place(&mut self) -> io::Result<()> {
        Ok(())
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
    pub fn len(&self) -> usize {
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

impl From<&ReadOnlyPieceTree> for Vec<u8> {
    fn from(pt: &ReadOnlyPieceTree) -> Self {
        let mut bytes = Vec::with_capacity(pt.len());
        let mut chunks = pt.chunks();
        let mut pos_chunk = chunks.get();

        while let Some((_pos, chunk)) = pos_chunk {
            let chunk_bytes = chunk.as_ref();
            bytes.extend_from_slice(chunk_bytes);
            pos_chunk = chunks.next();
        }

        bytes
    }
}
