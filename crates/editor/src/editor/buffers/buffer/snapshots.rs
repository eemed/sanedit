use sanedit_buffer::PieceTreeView;

use crate::editor::windows::Cursors;

#[derive(Debug, Clone, Default)]
pub(crate) struct SnapshotAux {
    pub(crate) cursors: Cursors,
    pub(crate) view_offset: u64,
}

pub(crate) type SnapshotId = usize;

/// Snapshots contains snapshots of buffer contents that can be used as undo and
/// redo points.
#[derive(Debug)]
pub(crate) struct Snapshots {
    current: Option<SnapshotId>,
    snapshots: Vec<SnapshotNode>,
}

impl Snapshots {
    pub fn new() -> Snapshots {
        Snapshots {
            current: None,
            snapshots: vec![],
        }
    }

    /// Remove current snapshot called in case something went wrong when
    /// applying changes
    pub fn remove_current_and_set(&mut self, id: Option<SnapshotId>) {
        if let Some(cur) = self.current {
            let current = self.snapshots.remove(cur);
            // Cut links
            for prev in &current.previous {
                self.snapshots[*prev].next.retain(|n| *n != cur);
            }
        }
        // Restore
        self.current = id;
    }

    pub fn current(&self) -> Option<SnapshotId> {
        self.current
    }

    pub fn insert(&mut self, snapshot: PieceTreeView) -> SnapshotId {
        let next_pos = self.snapshots.len();
        let mut node = SnapshotNode::new(snapshot, next_pos);
        if let Some(cur) = self.current {
            node.previous.push(cur);
            self.snapshots[cur].next.push(next_pos);
        }
        self.snapshots.push(node);
        self.current = Some(next_pos);
        next_pos
    }

    fn undo_pos(&self) -> Option<SnapshotId> {
        let node = self.snapshots.get(self.current?)?;
        node.previous.iter().max().cloned()
    }

    pub fn undo(&mut self) -> Option<SnapshotNode> {
        let latest = self.undo_pos()?;
        let node = self.snapshots.get(latest)?;
        self.current = Some(node.id);
        node.clone().into()
    }

    fn redo_pos(&self) -> Option<SnapshotId> {
        let node = self.snapshots.get(self.current?)?;
        node.next.iter().max().cloned()
    }

    pub fn redo(&mut self) -> Option<SnapshotNode> {
        let latest = self.redo_pos()?;
        let node = self.snapshots.get(latest)?;
        self.current = Some(node.id);
        node.clone().into()
    }

    pub fn aux_mut(&mut self, id: SnapshotId) -> Option<&mut SnapshotAux> {
        let node = self.snapshots.get_mut(id)?;
        Some(&mut node.data)
    }

    pub fn aux(&self, id: SnapshotId) -> Option<&SnapshotAux> {
        let node = self.snapshots.get(id)?;
        Some(&node.data)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SnapshotNode {
    pub(crate) id: SnapshotId,
    pub(crate) snapshot: PieceTreeView,
    previous: Vec<SnapshotId>,
    next: Vec<SnapshotId>,

    /// Extra data we can save to a snapshot
    pub(crate) data: SnapshotAux,
}

impl SnapshotNode {
    pub fn new(snapshot: PieceTreeView, id: SnapshotId) -> SnapshotNode {
        SnapshotNode {
            id,
            snapshot,
            previous: vec![],
            next: vec![],

            data: SnapshotAux::default(),
        }
    }
}
