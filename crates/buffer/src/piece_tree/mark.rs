use super::buffers::BufferKind;

/// A mark that tracks a position in text.
/// It can be retrieved anytime if the position has not been deleted
#[derive(Debug, Clone, Copy)]
pub struct Mark {
    pub(crate) orig: u64,
    pub(crate) kind: BufferKind,
    pub(crate) pos: u64,
    pub(crate) count: u32,
    pub(crate) after: bool,
}
