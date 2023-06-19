use std::ops::Range;

use super::ByteSlice;

#[derive(Debug)]
pub struct AddBuffer {}

impl AddBuffer {
    /// Append to add buffer.
    pub fn append(&self, bytes: &[u8]) {}

    /// Append to add buffer.
    /// This will only append the amount we can guarantee are contiguous.
    /// This will ensure you can slice the buffer from these points later using
    /// slice, and no copying will be done.
    ///
    /// This is used to create separate pieces in the tree when the data cannot be
    /// contiguous in memory.
    pub fn append_contiguous(&self, bytes: &[u8]) -> usize {
        todo!()
    }

    pub fn slice<'a>(&'a self, range: Range<usize>) -> ByteSlice<'a> {
        todo!()
    }
}
