use std::rc::Rc;

use sanedit_buffer::utf8::EndOfLine;
use sanedit_utils::sorted_vec::SortedVec;

use crate::editor::buffers::{BufferRange, SnapshotId};

#[derive(Debug, Default)]
pub(crate) struct ChangeResult {
    pub(crate) created_snapshot: Option<SnapshotId>,
    /// If kind is undo or redo, the restored snapshot id
    pub(crate) restored_snapshot: Option<SnapshotId>,
}

#[derive(Debug, Clone)]
pub(crate) struct Changes {
    changes: SortedVec<Change>,
}

impl Changes {
    pub fn new(changes: &[Change]) -> Changes {
        Changes {
            changes: SortedVec::from(changes),
        }
    }

    pub fn undo() -> Changes {
        let mut changes = SortedVec::new();
        changes.push(Change::undo());
        Changes { changes }
    }

    pub fn redo() -> Changes {
        let mut changes = SortedVec::new();
        changes.push(Change::redo());
        Changes { changes }
    }

    pub fn multi_remove(ranges: &[BufferRange]) -> Changes {
        let changes: Vec<Change> = ranges
            .iter()
            .map(|range| Change::remove(range.clone()))
            .collect();
        Changes::from(changes)
    }

    pub fn multi_insert(positions: &[u64], text: &[u8]) -> Changes {
        let text = Rc::new(text.to_vec());
        let changes: Vec<Change> = positions
            .iter()
            .map(|pos| Change::insert_rc(*pos, text.clone()))
            .collect();
        Changes::from(changes)
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Change> {
        self.changes.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }

    pub fn kind(&self) -> ChangesKind {
        let is_insert = self.changes.iter().all(Change::is_insert);
        if is_insert {
            return ChangesKind::Insert;
        }

        let is_remove = self.changes.iter().all(Change::is_remove);
        if is_remove {
            return ChangesKind::Remove;
        }

        let is_replace = self.changes.iter().all(Change::is_replace);
        if is_replace {
            return ChangesKind::Replace;
        }

        let is_undo = self.changes.iter().all(Change::is_undo);
        if is_undo {
            return ChangesKind::Undo;
        }

        let is_redo = self.changes.iter().all(Change::is_redo);
        if is_redo {
            return ChangesKind::Redo;
        }

        ChangesKind::Mixed
    }

    /// If all changes insert same text at different positions
    pub fn is_multi_insert(&self) -> bool {
        let mut text = None;
        for change in self.changes.iter() {
            if !change.is_insert() {
                return false;
            }

            match text {
                Some(old) => {
                    if old != &change.text {
                        return false;
                    }
                }
                None => text = Some(&change.text),
            }
        }

        true
    }

    pub fn is_remove(&self) -> bool {
        self.changes.iter().all(Change::is_remove)
    }

    pub fn is_insert_eol(&self) -> bool {
        self.changes.iter().all(Change::is_eol)
    }

    pub fn after_ranges(&self) -> Vec<BufferRange> {
        let mut ranges = Vec::with_capacity(self.changes.len());

        for change in self.changes.iter() {
            let start = change.start;
            let mut end = change.end;

            if start != end && !change.text.is_empty() {
                // Replace
                end = start + change.text.len() as u64;
            } else if change.text.is_empty() {
                // Remove
                end = start;
            } else {
                // Insert
                end += change.text.len() as u64;
            }
            ranges.push(start..end);
        }

        ranges
    }

    pub fn before_ranges(&self) -> Vec<BufferRange> {
        let mut ranges = Vec::with_capacity(self.changes.len());

        for change in self.changes.iter() {
            let start = change.start;
            let end = change.end;
            ranges.push(start..end);
        }

        ranges
    }

    pub fn needs_undo_point(&self, previous: Option<&Changes>) -> bool {
        // no previous edits, undo point should be created automatically
        if previous.is_none() {
            return false;
        }

        use ChangesKind::*;

        let pchange = previous.unwrap();
        match (pchange.kind(), self.kind()) {
            (Insert, Insert) => {
                if self.is_insert_eol() {
                    return true;
                }

                let pranges = pchange.after_ranges();
                let ranges = self.before_ranges();

                if pranges.len() != ranges.len() {
                    return true;
                }

                let mut ins = 0;
                for i in 0..pranges.len() {
                    let prang = &pranges[i];
                    let crang = &ranges[i];

                    if prang.end + ins != crang.start {
                        return true;
                    }

                    ins += prang.end - prang.start;
                }

                false
            }
            (Remove, Remove) => {
                let pranges = pchange.after_ranges();
                let ranges = self.before_ranges();

                if pranges.len() != ranges.len() {
                    return true;
                }

                let mut rem = 0;
                for i in 0..pranges.len() {
                    let prang = &pranges[i];
                    let crang = &ranges[i];

                    if prang.start != crang.end + rem {
                        return true;
                    }

                    rem += prang.end - prang.start;
                }

                false
            }
            (_, Redo) => false,
            (Redo | Undo, _) => false,
            _ => true,
        }
    }
}

impl From<Vec<Change>> for Changes {
    fn from(value: Vec<Change>) -> Self {
        Changes {
            changes: SortedVec::from(value),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Change {
    // Range stored separately as Range<u64> does not implement ord
    /// Inclusive
    start: u64,
    /// Exclusive
    end: u64,
    text: Rc<Vec<u8>>,
    flags: Flags,
}

impl Change {
    fn undo() -> Change {
        Change {
            start: 0,
            end: 0,
            text: Rc::new(Vec::new()),
            flags: Flags::UNDO,
        }
    }

    fn redo() -> Change {
        Change {
            start: 0,
            end: 0,
            text: Rc::new(Vec::new()),
            flags: Flags::REDO,
        }
    }

    pub fn insert(at: u64, text: &[u8]) -> Change {
        Change {
            start: at,
            end: at,
            text: Rc::new(text.into()),
            flags: Flags::default(),
        }
    }

    fn insert_rc(at: u64, text: Rc<Vec<u8>>) -> Change {
        Change {
            start: at,
            end: at,
            text,
            flags: Flags::default(),
        }
    }

    pub fn remove(range: BufferRange) -> Change {
        Change {
            start: range.start,
            end: range.end,
            text: Rc::new(Vec::new()),
            flags: Flags::default(),
        }
    }

    pub fn replace(range: BufferRange, text: &[u8]) -> Change {
        Change {
            start: range.start,
            end: range.end,
            text: Rc::new(text.into()),
            flags: Flags::default(),
        }
    }

    pub fn range(&self) -> BufferRange {
        self.start..self.end
    }

    pub fn start(&self) -> u64 {
        self.start
    }

    pub fn end(&self) -> u64 {
        self.end
    }

    pub fn text(&self) -> &[u8] {
        &self.text
    }

    pub fn is_undo(&self) -> bool {
        self.flags.contains(Flags::UNDO)
    }

    pub fn is_redo(&self) -> bool {
        self.flags.contains(Flags::REDO)
    }

    pub fn is_remove(&self) -> bool {
        self.text.is_empty() && self.start != self.end
    }

    pub fn is_insert(&self) -> bool {
        !self.text.is_empty() && self.start == self.end
    }

    pub fn is_replace(&self) -> bool {
        !self.text.is_empty() && self.start != self.end
    }

    pub fn is_eol(&self) -> bool {
        EndOfLine::is_eol(self.text.as_ref())
    }
}

bitflags::bitflags! {
    #[derive(Default)]
    struct Flags: u8 {
        const UNDO = 0b00000001;
        const REDO = 0b00000010;
    }
}

#[derive(Debug, Clone)]
pub(crate) enum ChangesKind {
    Insert,
    Remove,
    Replace,
    Undo,
    Redo,
    Mixed,
}
