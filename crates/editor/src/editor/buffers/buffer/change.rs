use crate::editor::buffers::SnapshotId;

#[derive(Debug, Default)]
pub(crate) struct ChangeResult {
    /// If a new snapshot was created because of the change
    pub(crate) created_snapshot: Option<SnapshotId>,
    /// If kind is undo or redo, the restored snapshot id
    pub(crate) restored_snapshot: Option<SnapshotId>,

    /// If operation forks a snapshot ie. undo and insert something else
    pub(crate) forked_snapshot: Option<SnapshotId>,
}
