use std::{ops::Range, sync::Arc};

use crate::piece_tree::buffers::ByteSlice;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OriginalBufferSlice {
    pub(crate) ptr: Arc<[u8]>,
    pub(crate) offset: usize,
    pub(crate) len: usize,
}

impl AsRef<[u8]> for OriginalBufferSlice {
    fn as_ref(&self) -> &[u8] {
        &self.ptr[self.offset..self.offset + self.len]
    }
}

impl OriginalBufferSlice {
    pub fn slice(&mut self, range: Range<usize>) {
        self.offset += range.start;
        self.len = range.len();
    }
}

impl<'a> From<OriginalBufferSlice> for ByteSlice<'a> {
    fn from(value: OriginalBufferSlice) -> Self {
        ByteSlice::Original(value)
    }
}
