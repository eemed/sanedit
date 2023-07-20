use std::time;

use sanedit_buffer::ReadOnlyPieceTree;

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

    pub fn undo(&mut self) -> Option<SnapshotNode> {
        let node = self.snapshots.get(self.current)?;
        // Latest has largest index
        let latest = node.previous.iter().max()?;
        let node = self.snapshots.get(*latest)?;
        self.current = node.idx;
        node.clone().into()
    }

    pub fn redo(&mut self) -> Option<SnapshotNode> {
        let node = self.snapshots.get(self.current)?;
        // Latest has largest index
        let latest = node.next.iter().max()?;
        let node = self.snapshots.get(*latest)?;
        self.current = node.idx;
        node.clone().into()
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SnapshotNode {
    pub(crate) idx: usize,
    pub(crate) snapshot: ReadOnlyPieceTree,
    pub(crate) timestamp: time::Instant,
    previous: Vec<usize>,
    next: Vec<usize>,
}

impl SnapshotNode {
    pub fn new(snapshot: ReadOnlyPieceTree, idx: usize) -> SnapshotNode {
        SnapshotNode {
            idx,
            snapshot,
            timestamp: time::Instant::now(),
            previous: vec![],
            next: vec![],
        }
    }
}
