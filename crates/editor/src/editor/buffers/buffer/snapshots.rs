use sanedit_buffer::ReadOnlyPieceTree;

use crate::editor::windows::Cursors;

#[derive(Debug, Clone)]
pub(crate) struct SnapshotData {
    pub(crate) cursors: Cursors,
    pub(crate) view_offset: usize,
}

/// Snapshots contains snapshots of buffer contents that can be used as undo and
/// redo points.
#[derive(Debug)]
pub(crate) struct Snapshots {
    current: usize,
    snapshots: Vec<SnapshotNode>,
}

impl Snapshots {
    pub fn new(initial: ReadOnlyPieceTree) -> Snapshots {
        Snapshots {
            current: 0,
            snapshots: vec![SnapshotNode::new(initial, 0)],
        }
    }

    pub fn insert(&mut self, snapshot: ReadOnlyPieceTree) {
        let next_idx = self.snapshots.len();
        let mut node = SnapshotNode::new(snapshot, next_idx);
        node.previous.push(self.current);
        self.snapshots[self.current].next.push(next_idx);
        self.snapshots.push(node);
        self.current = next_idx;
    }

    fn undo_index(&self) -> Option<usize> {
        let node = self.snapshots.get(self.current)?;
        node.previous.iter().max().cloned()
    }

    pub fn undo(&mut self) -> Option<SnapshotNode> {
        let latest = self.undo_index()?;
        let node = self.snapshots.get(latest)?;
        self.current = node.idx;
        node.clone().into()
    }

    fn redo_index(&self) -> Option<usize> {
        let node = self.snapshots.get(self.current)?;
        node.next.iter().max().cloned()
    }

    pub fn redo(&mut self) -> Option<SnapshotNode> {
        let latest = self.redo_index()?;
        let node = self.snapshots.get(latest)?;
        self.current = node.idx;
        node.clone().into()
    }

    pub fn has_redo(&self) -> bool {
        self.redo_index().is_some()
    }

    pub fn has_undo(&self) -> bool {
        self.undo_index().is_some()
    }

    pub fn set_current_data(&mut self, sdata: SnapshotData) {
        self.snapshots[self.current].data = Some(sdata);
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SnapshotNode {
    pub(crate) idx: usize,
    pub(crate) snapshot: ReadOnlyPieceTree,
    previous: Vec<usize>,
    next: Vec<usize>,

    /// Extra data we can save to a snapshot
    pub(crate) data: Option<SnapshotData>,
}

impl SnapshotNode {
    pub fn new(snapshot: ReadOnlyPieceTree, idx: usize) -> SnapshotNode {
        SnapshotNode {
            idx,
            snapshot,
            previous: vec![],
            next: vec![],

            data: None,
        }
    }
}
