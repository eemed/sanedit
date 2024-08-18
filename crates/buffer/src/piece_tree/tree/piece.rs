use std::{mem, ops::Range};

use crate::piece_tree::buffers::BufferKind;

/// Piece describes an index and byte length in a buffer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Piece {
    /// are we indexing add buffer or read only buffer
    pub(crate) kind: BufferKind,

    /// index in buffer
    pub(crate) pos: u64,
    /// Length in bytes
    pub(crate) len: u64,

    /// This is used when the same buffer part is used multiple times, so kind, pos, and len
    /// are the same. count can be used to identify a piece from other same
    /// pieces allowing piece to represent the same region but with its own id.
    pub(crate) count: u32,
}

impl Piece {
    pub fn new(kind: BufferKind, pos: u64, len: u64) -> Self {
        Piece {
            kind,
            pos,
            len,
            count: 0,
        }
    }

    pub fn new_with_count(kind: BufferKind, pos: u64, len: u64, count: u32) -> Self {
        Piece {
            kind,
            pos,
            len,
            count,
        }
    }

    /// Split the piece at offset from the piece start.
    /// Modifies the current piece to be the left half
    /// and returns the right half.
    pub fn split_left(&mut self, offset: u64) -> Piece {
        debug_assert!(offset <= self.len);
        let right_start = self.pos + offset;
        let right_len = self.len - offset;

        self.len = offset;

        Piece::new_with_count(self.kind, right_start, right_len, self.count)
    }

    /// Split the piece at offset from the piece start.
    /// Modifies the current piece to be the right half
    /// and returns the left half.
    pub fn split_right(&mut self, offset: u64) -> Piece {
        let right = self.split_left(offset);
        mem::replace(self, right)
    }

    /// Returns the range this piece references
    pub fn range(&self) -> Range<u64> {
        self.pos..self.pos + self.len
    }
}
