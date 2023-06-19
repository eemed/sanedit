mod add;
mod original;

use std::borrow::Cow;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum BufferKind {
    Add,
    Original,
}

pub(crate) type ByteSlice<'a> = Cow<'a, [u8]>;
// pub(crate) type AddBuffer = Vec<u8>;

pub(crate) use add::AddBuffer;
pub(crate) use original::OriginalBuffer;
