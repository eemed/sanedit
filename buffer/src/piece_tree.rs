pub(crate) mod buffers;
pub(crate) mod builder;
// pub(crate) mod bytes;
// pub(crate) mod chunks;
// mod graphemes;
// mod slice;
pub(crate) mod tree;

use std::fs::File;
use std::io::{self, Write};
use std::ops::{Bound, RangeBounds};

// use self::slice::PieceTreeSlice;
use self::tree::pieces::Pieces;
use self::tree::Tree;
use crate::piece_tree::buffers::BufferKind;
// use crate::piece_tree::chunks::Chunks;
use crate::piece_tree::tree::piece::Piece;
use buffers::AddBuffer;
use buffers::OriginalBuffer;

pub use crate::cursor_iterator::CursorIterator;
// pub use crate::piece_tree::bytes::Bytes;
pub use builder::PieceTreeBuilder;
// pub use graphemes::{next_grapheme, next_grapheme_boundary, prev_grapheme, prev_grapheme_boundary};

/// A Snapshot of the piece tree.
//
// Takes a snapshot of the current tree.
// It can be restored assuming the snapshot was taken from the same piece tree.
#[derive(Clone, Debug)]
pub struct Snapshot {
    pub(crate) tree: Tree,
    pub(crate) len: usize,
}

/// A mark that tracks a position in text.
/// It can be retrieved anytime if the position has not been deleted
#[derive(Debug, Clone, Copy)]
pub struct Mark {
    pub(crate) kind: BufferKind,
    pub(crate) pos: usize,
}

/// Byte buffer
///
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
    pub(crate) orig: OriginalBuffer,
    pub(crate) add: AddBuffer,
    pub(crate) tree: Tree,
    pub(crate) len: usize,
}

impl PieceTree {
    /// Create a new empty piece tree
    #[inline]
    pub fn new() -> PieceTree {
        let orig_buf = OriginalBuffer::new();
        Self::from_original_buffer(orig_buf).unwrap()
    }

    /// Create a new piece tree from a reader.
    /// The content is stored in memory.
    #[inline]
    pub fn from_reader<R: io::Read>(reader: R) -> io::Result<PieceTree> {
        let orig_buf = OriginalBuffer::from_reader(reader)?;
        Self::from_original_buffer(orig_buf)
    }

    /// Create a new piece tree from a file.
    /// The whole file is not read into memory at any time.
    /// Windowing is used instead which only reads a part of the file.
    #[inline]
    pub fn from_file<F>(file: File) -> io::Result<PieceTree> {
        let orig_buf = OriginalBuffer::from_file(file)?;
        Self::from_original_buffer(orig_buf)
    }

    /// Take a snapshot.
    #[inline]
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            tree: self.tree.clone(),
            len: self.len,
        }
    }

    /// Restore the buffer to a snapshot
    #[inline]
    pub fn snapshot_restore(&mut self, snapshot: Snapshot) {
        self.tree = snapshot.tree;
        self.len = snapshot.len;
    }

    /// Mark a position in the buffer
    // Internally works using offsets into the read only and append only buffers.
    // These can be safely indexed into because they never change after written.
    // Searching for a mark is O(n) operation where n is the number of pieces in the
    // piece tree
    pub fn mark(&self, pos: usize) -> Mark {
        debug_assert!(
            pos <= self.len,
            "mark: Attempting to index {} over buffer len {}",
            pos,
            self.len
        );
        let pieces = Pieces::new(self, pos, 0..self.len);
        let (p_pos, piece) = pieces
            .get()
            .expect(&format!("Cannot find a piece for position {pos}"));
        let off = pos - p_pos;
        Mark {
            kind: piece.kind,
            pos: piece.pos + off,
        }
    }

    /// Get a buffer position from a mark.
    /// If the buffer position has been deleted returns None.
    pub fn mark_to_pos(&self, mark: &Mark) -> Option<usize> {
        let mut pieces = Pieces::new(self, 0, 0..self.len);
        let mut piece = pieces.get();

        while let Some((p_pos, p)) = piece {
            if p.kind == mark.kind && p.pos <= mark.pos && mark.pos < p.pos + p.len {
                let off = mark.pos - p.pos;
                return Some(p_pos + off);
            }
            piece = pieces.next();
        }

        None
    }

    pub fn write_to<W: Write>(&self, mut writer: W) -> io::Result<usize> {
        todo!()
        // let mut written = 0;
        // let mut chunks = self.str_chunks();
        // let mut chunk = chunks.current();

        // while let Some(chk) = chunk {
        //     writer.write(chk.as_bytes())?;
        //     written += chk.len();
        //     chunk = chunks.next();
        // }

        // writer.flush()?;
        // Ok(written)
    }

    #[inline]
    fn from_original_buffer(orig_buf: OriginalBuffer) -> io::Result<PieceTree> {
        let add_buf = AddBuffer::new();
        let mut pieces = Tree::new();

        let ob_len = orig_buf.len();
        if ob_len > 0 {
            let piece = Piece::new(BufferKind::Original, 0, ob_len);
            pieces.insert(0, piece);
        }

        let pt = PieceTree {
            orig: orig_buf,
            add: add_buf,
            tree: pieces,
            len: ob_len,
        };

        Ok(pt)
    }

    #[inline]
    pub fn insert_str(&mut self, pos: usize, string: &str) {
        self.insert(pos, string.as_bytes());
    }

    pub fn insert<B: AsRef<[u8]>>(&mut self, pos: usize, bytes: B) {
        debug_assert!(
            pos <= self.len,
            "insert: Attempting to index {} over buffer len {}",
            pos,
            self.len
        );

        let bytes = bytes.as_ref();
        if bytes.is_empty() {
            return;
        }

        let bpos = self.add.len();
        self.add.extend_from_slice(bytes);

        let piece = Piece::new(BufferKind::Add, bpos, bytes.len());
        self.len += piece.len;

        self.tree.insert(pos, piece);
    }

    #[inline]
    pub fn insert_char(&mut self, pos: usize, ch: char) {
        let mut buf = [0; 4];
        let string = ch.encode_utf8(&mut buf);
        self.insert_str(pos, string);
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
            std::ops::Bound::Unbounded => self.len,
        };

        debug_assert!(
            end <= self.len,
            "remove: Attempting to index {} over buffer len {}",
            end,
            self.len
        );

        self.tree.remove(start..end);
        self.len -= end - start;
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

    #[inline]
    pub fn append(&mut self, bytes: &[u8]) {
        self.insert(self.len, bytes);
    }

    // #[inline]
    // pub fn bytes(&self) -> Bytes {
    //     self.bytes_at(0)
    // }

    // #[inline]
    // pub fn bytes_at(&self, pos: usize) -> Bytes {
    //     debug_assert!(
    //         pos <= self.len,
    //         "bytes_at: Attempting to index {} over buffer len {}",
    //         pos,
    //         self.len
    //     );
    //     Bytes::new(self, pos)
    // }

    // #[inline]
    // pub fn chunks(&self) -> Chunks {
    //     self.chunks_at(0)
    // }

    // #[inline]
    // pub fn chunks_at(&self, pos: usize) -> Chunks {
    //     debug_assert!(
    //         pos <= self.len,
    //         "chunks_at: Attempting to index {} over buffer len {}",
    //         pos,
    //         self.len
    //     );
    //     Chunks::new(self, pos)
    // }

    // #[inline]
    // pub fn slice<'a, R: RangeBounds<usize>>(&'a self, range: R) -> PieceTreeSlice<'a> {
    //     let start = match range.start_bound() {
    //         Bound::Included(n) => *n,
    //         Bound::Excluded(n) => *n + 1,
    //         Bound::Unbounded => 0,
    //     };

    //     let end = match range.end_bound() {
    //         Bound::Included(n) => *n + 1,
    //         Bound::Excluded(n) => *n,
    //         Bound::Unbounded => self.len,
    //     };

    //     PieceTreeSlice::new(self, start, end)
    // }
}

impl From<&PieceTree> for String {
    fn from(pt: &PieceTree) -> Self {
        todo!()
        // let mut result = String::with_capacity(pt.len);
        // let mut chunks = pt.str_chunks();
        // let mut chunk = chunks.current();

        // while let Some(chk) = chunk {
        //     result.push_str(&chk);

        //     chunk = chunks.next();
        // }

        // result
    }
}

impl Default for PieceTree {
    fn default() -> Self {
        PieceTree::new()
    }
}

// #[cfg(test)]
// mod test {
//     use super::*;

//     #[test]
//     fn graphemes_at() {
//         let mut pt = PieceTreeBytes::new_empty();
//         pt.insert_str(0, "§");

//         pt.graphemes_at(1);
//     }
// }
