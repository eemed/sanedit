mod add;
mod original;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum BufferKind {
    Add,
    Original,
}

// pub(crate) type ByteSlice<'a> = Cow<'a, [u8]>;

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

impl<'a> From<OriginalBufferSlice> for ByteSlice<'a> {
    fn from(value: OriginalBufferSlice) -> Self {
        ByteSlice::Original(value)
    }
}

pub(crate) use add::{
    create_add_buffer_reader_writer, AddBufferReader, AddBufferWriter, AppendResult,
};
pub(crate) use original::OriginalBuffer;

use self::original::OriginalBufferSlice;
