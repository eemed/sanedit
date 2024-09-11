use crate::editor::buffers::SnapshotId;

#[derive(Debug, Default)]
pub(crate) struct ChangeResult {
    pub(crate) created_snapshot: Option<SnapshotId>,
    /// If kind is undo or redo, the restored snapshot id
    pub(crate) restored_snapshot: Option<SnapshotId>,
}
