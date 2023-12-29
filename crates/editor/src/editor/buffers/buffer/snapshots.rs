use sanedit_buffer::ReadOnlyPieceTree;

use crate::editor::windows::Cursors;

#[derive(Debug, Clone)]
pub(crate) struct SnapshotData {
    pub(crate) cursors: Cursors,
    pub(crate) view_offset: usize,
}

pub(crate) type SnapshotId = usize;

/// Snapshots contains snapshots of buffer contents that can be used as undo and
/// redo points.
#[derive(Debug)]
pub(crate) struct Snapshots {
    current: SnapshotId,
    snapshots: Vec<SnapshotNode>,
}

impl Snapshots {
    pub fn new(initial: ReadOnlyPieceTree) -> Snapshots {
        Snapshots {
            current: 0,
            snapshots: vec![SnapshotNode::new(initial, 0)],
        }
    }

    pub fn insert(&mut self, snapshot: ReadOnlyPieceTree) -> SnapshotId {
        let next_pos = self.snapshots.len();
        let mut node = SnapshotNode::new(snapshot, next_pos);
        node.previous.push(self.current);
        self.snapshots[self.current].next.push(next_pos);
        self.snapshots.push(node);
        self.current = next_pos;
        self.current
    }

    fn undo_pos(&self) -> Option<SnapshotId> {
        let node = self.snapshots.get(self.current)?;
        node.previous.iter().max().cloned()
    }

    pub fn undo(&mut self) -> Option<SnapshotNode> {
        let latest = self.undo_pos()?;
        let node = self.snapshots.get(latest)?;
        self.current = node.id;
        node.clone().into()
    }

    fn redo_pos(&self) -> Option<SnapshotId> {
        let node = self.snapshots.get(self.current)?;
        node.next.iter().max().cloned()
    }

    pub fn redo(&mut self) -> Option<SnapshotNode> {
        let latest = self.redo_pos()?;
        let node = self.snapshots.get(latest)?;
        self.current = node.id;
        node.clone().into()
    }

    pub fn has_redo(&self) -> bool {
        self.redo_pos().is_some()
    }

    pub fn has_undo(&self) -> bool {
        self.undo_pos().is_some()
    }

    pub fn set_data(&mut self, id: SnapshotId, sdata: SnapshotData) {
        if let Some(snap) = self.snapshots.get_mut(id) {
            snap.data = Some(sdata);
        }
    }

    pub fn get_data(&self, id: SnapshotId) -> Option<SnapshotData> {
        let node = self.snapshots.get(id)?;
        node.data.clone()
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SnapshotNode {
    pub(crate) id: SnapshotId,
    pub(crate) snapshot: ReadOnlyPieceTree,
    previous: Vec<SnapshotId>,
    next: Vec<SnapshotId>,

    /// Extra data we can save to a snapshot
    pub(crate) data: Option<SnapshotData>,
}

impl SnapshotNode {
    pub fn new(snapshot: ReadOnlyPieceTree, id: SnapshotId) -> SnapshotNode {
        SnapshotNode {
            id,
            snapshot,
            previous: vec![],
            next: vec![],

            data: None,
        }
    }
}
