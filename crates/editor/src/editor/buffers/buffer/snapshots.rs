use std::time;

use sanedit_buffer::piece_tree::Snapshot;

/// Snapshots contains snapshots of buffer contents that can be used as undo and
/// redo points.
#[derive(Debug)]
pub(crate) struct Snapshots {
    current: usize,
    snapshots: Vec<Node>,
}

impl Snapshots {
    pub fn new(initial: Snapshot) -> Snapshots {
        Snapshots {
            current: 0,
            snapshots: vec![Node::new(initial, 0)],
        }
    }

    pub fn insert(&mut self, snapshot: Snapshot) {
        let next_idx = self.snapshots.len();
        let mut node = Node::new(snapshot, next_idx);
        node.children.push(self.current);
        self.snapshots[self.current].parents.push(next_idx);
        self.snapshots.push(node);
        self.current = next_idx;
    }

    pub fn undo(&mut self) -> Option<Snapshot> {
        let node = self.snapshots.get(self.current)?;
        // Latest has largest index
        let latest = node.children.iter().max()?;
        self.snapshots.get(*latest).map(|n| n.snapshot.clone())
    }

    pub fn redo(&mut self) -> Option<Snapshot> {
        let node = self.snapshots.get(self.current)?;
        // Latest has largest index
        let latest = node.parents.iter().max()?;
        self.snapshots.get(*latest).map(|n| n.snapshot.clone())
    }
}

#[derive(Debug, Clone)]
struct Node {
    pub(crate) idx: usize,
    pub(crate) snapshot: Snapshot,
    pub(crate) timestamp: time::Instant,
    children: Vec<usize>,
    parents: Vec<usize>,
}

impl Node {
    pub fn new(snapshot: Snapshot, idx: usize) -> Node {
        Node {
            idx,
            snapshot,
            timestamp: time::Instant::now(),
            children: vec![],
            parents: vec![],
        }
    }
}
