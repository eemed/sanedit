mod add;
mod original;

pub(crate) use add::{AddBuffer, AddBufferReader, AddBufferWriter, AppendResult};
pub(crate) use original::OriginalBuffer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BufferKind {
    Add,
    Original,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ByteSlice<'a>(&'a [u8]);

impl<'a> AsRef<[u8]> for ByteSlice<'a> {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl<'a> From<&'a [u8]> for ByteSlice<'a> {
    fn from(value: &'a [u8]) -> Self {
        ByteSlice(value)
    }
}
