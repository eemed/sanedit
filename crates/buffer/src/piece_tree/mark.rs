use super::buffers::BufferKind;

/// A mark that tracks a position in text.
/// It can be retrieved if the position has not been deleted
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mark {
    pub(crate) orig: u64,
    pub(crate) kind: BufferKind,
    pub(crate) pos: u64,
    pub(crate) count: u32,
    pub(crate) end_of_buffer: bool,
}

impl Mark {
    pub fn original_position(&self) -> u64 {
        self.orig
    }
}

#[derive(Debug, Clone, Copy)]
pub enum MarkResult {
    Deleted(u64),
    Found(u64),
}

impl MarkResult {
    pub fn pos(&self) -> u64 {
        match self {
            MarkResult::Deleted(n) => *n,
            MarkResult::Found(n) => *n,
        }
    }

    pub fn is_found(&self) -> bool {
        matches!(self, MarkResult::Found(..))
    }
}
