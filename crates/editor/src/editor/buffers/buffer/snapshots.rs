use std::time::Instant;

use sanedit_buffer::{Mark, PieceTreeSlice};

use crate::editor::windows::Cursors;

#[derive(Debug, Clone, Default)]
pub(crate) struct SavedWindowState {
    pub(crate) cursors: Cursors,
    pub(crate) view_offset: u64,
    pub(crate) change_start: Option<Mark>,
    pub(crate) last_selection: Option<Cursors>,
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

    pub fn nodes(&self) -> &[SnapshotNode] {
        &self.snapshots
    }

    pub fn get(&self, id: SnapshotId) -> Option<&SnapshotNode> {
        self.snapshots.get(id)
    }

    pub fn goto_get(&mut self, id: SnapshotId) -> Option<&SnapshotNode> {
        let node = self.snapshots.get(id)?;
        self.current = Some(id);
        Some(node)
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

    pub fn insert(&mut self, snapshot: PieceTreeSlice) -> SnapshotId {
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

    pub fn undo(&mut self) -> Option<&SnapshotNode> {
        let latest = self.undo_pos()?;
        let node = self.snapshots.get(latest)?;
        self.current = Some(node.id);
        node.into()
    }

    fn redo_pos(&self) -> Option<SnapshotId> {
        let node = self.snapshots.get(self.current?)?;
        node.next.iter().max().cloned()
    }

    pub fn redo(&mut self) -> Option<&SnapshotNode> {
        let latest = self.redo_pos()?;
        let node = self.snapshots.get(latest)?;
        self.current = Some(node.id);
        node.into()
    }

    pub fn window_state_mut(&mut self, id: SnapshotId) -> Option<&mut SavedWindowState> {
        let node = self.snapshots.get_mut(id)?;
        Some(&mut node.data)
    }

    pub fn window_state(&self, id: SnapshotId) -> Option<&SavedWindowState> {
        let node = self.snapshots.get(id)?;
        Some(&node.data)
    }

    pub fn next_of(&self, id: SnapshotId) -> Option<SnapshotId> {
        for node in &self.snapshots {
            if node.id == id {
                let max = node.next.iter().max()?;
                let node = self.snapshots.get(*max)?;
                return Some(node.id);
            }
        }

        None
    }

    pub fn prev_of(&self, id: SnapshotId) -> Option<SnapshotId> {
        for node in &self.snapshots {
            if node.id == id {
                let max = node.previous.iter().max()?;
                let node = self.snapshots.get(*max)?;
                return Some(node.id);
            }
        }

        None
    }

    pub fn iter<'a>(&'a self) -> SnapshotIter<'a> {
        let root = if self.snapshots.is_empty() {
            None
        } else {
            Some(0)
        };

        SnapshotIter::new(&self.snapshots, root)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SnapshotNode {
    pub(crate) id: SnapshotId,
    pub(crate) snapshot: PieceTreeSlice,
    pub(crate) timestamp: Instant,

    pub(crate) previous: Vec<SnapshotId>,
    pub(crate) next: Vec<SnapshotId>,

    /// Extra data we can save to a snapshot
    pub(crate) data: SavedWindowState,
}

impl SnapshotNode {
    pub fn new(snapshot: PieceTreeSlice, id: SnapshotId) -> SnapshotNode {
        SnapshotNode {
            id,
            snapshot,
            timestamp: Instant::now(),
            previous: vec![],
            next: vec![],

            data: SavedWindowState::default(),
        }
    }
}

/// Iterate snapshots in the order they should be displayed on the UI
#[derive(Debug)]
pub(crate) struct SnapshotIter<'a> {
    stack: Vec<&'a SnapshotNode>,
}

impl<'a> SnapshotIter<'a> {
    pub fn new(snapshots: &'a [SnapshotNode], root: Option<SnapshotId>) -> SnapshotIter<'a> {
        let mut postorder = vec![];
        let mut stack = vec![];
        if let Some(root) = root {
            stack.push(root)
        }

        while let Some(n) = stack.pop() {
            let node = &snapshots[n];
            for child in node.next.iter().rev() {
                stack.push(*child);
            }

            postorder.push(&snapshots[n]);
        }

        SnapshotIter { stack: postorder }
    }
}

impl<'a> Iterator for SnapshotIter<'a> {
    type Item = &'a SnapshotNode;

    fn next(&mut self) -> Option<Self::Item> {
        self.stack.pop()
    }
}
