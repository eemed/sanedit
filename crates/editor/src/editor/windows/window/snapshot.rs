use crate::editor::buffers::{SavedWindowState, SnapshotId};

#[derive(Debug, Default)]
pub(crate) struct SnapshotView {
    pub(crate) selection: usize,
    pub(crate) show: bool,

    /// Buffer is changed if snapshot is previewed, this is to restore it
    pub(crate) original_buffer: Option<SnapshotId>,
}
