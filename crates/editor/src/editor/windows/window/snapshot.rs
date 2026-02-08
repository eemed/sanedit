use crate::editor::{buffers::SnapshotId, windows::Mouse};

#[derive(Debug, Default)]
pub(crate) struct SnapshotView {
    pub(crate) selection: usize,
    pub(crate) show: bool,
    pub(crate) mouse: Mouse,

    /// Buffer is changed if snapshot is previewed, this is to restore it
    pub(crate) original_buffer: Option<SnapshotId>,
}
