use std::mem;

use crate::piece_tree::buffers::BufferKind;

/// Piece describes an index and byte length in a buffer.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Piece {
    /// are we indexing add buffer or read only buffer
    pub(crate) kind: BufferKind,
    /// index in buffer
    pub(crate) pos: usize,
    /// Length in bytes
    pub(crate) len: usize,
}

impl Piece {
    pub fn new(buf_type: BufferKind, index: usize, len: usize) -> Self {
        Piece {
            kind: buf_type,
            pos: index,
            len,
        }
    }

    /// Split the piece at offset from the piece start.
    /// Modifies the current piece to be the left half
    /// and returns the right half.
    pub fn split_left(&mut self, offset: usize) -> Piece {
        debug_assert!(offset <= self.len);
        let right_start = self.pos + offset;
        let right_len = self.len - offset;

        self.len = offset;

        Piece::new(self.kind, right_start, right_len)
    }

    /// Split the piece at offset from the piece start.
    /// Modifies the current piece to be the right half
    /// and returns the left half.
    pub fn split_right(&mut self, offset: usize) -> Piece {
        let right = self.split_left(offset);
        mem::replace(self, right)
    }
}
