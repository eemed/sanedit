mod add;
mod original;

pub(crate) use add::{AddBuffer, AddBufferReader, AddBufferWriter, AppendResult};
pub(crate) use original::OriginalBuffer;

use self::original::OriginalBufferSlice;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BufferKind {
    Add,
    Original,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ByteSlice<'a> {
    Ref(&'a [u8]),
    Original(OriginalBufferSlice),
}

impl<'a> AsRef<[u8]> for ByteSlice<'a> {
    fn as_ref(&self) -> &[u8] {
        match self {
            ByteSlice::Ref(bytes) => bytes,
            ByteSlice::Original(orig) => orig.as_ref(),
        }
    }
}

impl<'a> From<&'a [u8]> for ByteSlice<'a> {
    fn from(value: &'a [u8]) -> Self {
        ByteSlice::Ref(value)
    }
}
