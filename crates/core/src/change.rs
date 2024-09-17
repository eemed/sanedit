use std::{borrow::Cow, rc::Rc};

use sanedit_buffer::{utf8::EndOfLine, PieceTree, PieceTreeView};
use sanedit_utils::sorted_vec::SortedVec;

use crate::{range::BufferRange, BufferRangeExt as _, Cursor};

#[derive(Debug)]
pub struct Edit {
    pub buf: PieceTreeView,
    pub changes: Changes,
}

#[derive(Debug, Clone)]
pub struct Changes {
    changes: SortedVec<Change>,
    flags: Flags,
}

impl Changes {
    pub fn new(changes: &[Change]) -> Changes {
        Changes {
            changes: SortedVec::from(changes),
            flags: Flags::default(),
        }
    }

    pub fn undo() -> Changes {
        Changes {
            changes: SortedVec::new(),
            flags: Flags::UNDO,
        }
    }

    pub fn redo() -> Changes {
        Changes {
            changes: SortedVec::new(),
            flags: Flags::REDO,
        }
    }

    pub fn is_undo(&self) -> bool {
        self.flags.contains(Flags::UNDO)
    }

    pub fn is_redo(&self) -> bool {
        self.flags.contains(Flags::REDO)
    }

    /// Apply the change and return whether anything changed.
    /// This wont apply undo or redo you should handle those yourself
    pub fn apply(&self, pt: &mut PieceTree) {
        if self.is_multi_insert() {
            let starts: Vec<u64> = self.changes.iter().map(|change| change.start()).collect();
            let text = self.changes.first().expect("No changes").text();
            pt.insert_multi(&starts, text);
            return;
        }

        let mut off = 0i128;

        for change in self.changes.iter() {
            let mut range = change.range();
            let abs = off.abs() as u64;
            if off.is_negative() {
                range.backward(abs);
            } else {
                range.forward(abs);
            }

            if range.len() != 0 {
                self.remove_ranges(pt, &[range]);
            }

            if !change.text().is_empty() {
                let abs = off.abs() as u64;
                let start = if off.is_negative() {
                    change.start() - abs
                } else {
                    change.start() + abs
                };
                pt.insert_multi(&[start], change.text());
            }

            off -= change.range().len() as i128;
            off += change.text().len() as i128;
        }
    }

    fn remove_ranges(&self, pt: &mut PieceTree, ranges: &[BufferRange]) {
        fn is_sorted(ranges: &[BufferRange]) -> bool {
            let mut last = 0;
            for range in ranges.iter() {
                if range.start < last {
                    return false;
                }

                last = range.end;
            }

            true
        }

        let ranges: Cow<[BufferRange]> = if is_sorted(ranges) {
            ranges.into()
        } else {
            let mut ranges = ranges.to_vec();
            ranges.sort_by(|a, b| a.start.cmp(&b.start));
            ranges.into()
        };

        for range in ranges.iter().rev() {
            pt.remove(range.clone());
        }
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

    pub fn disable_undo_point_creation(&mut self) {
        self.flags.insert(Flags::DISABLE_UNDO_POINT_CREATION);
    }

    pub fn allows_undo_point_creation(&self) -> bool {
        !self.flags.contains(Flags::DISABLE_UNDO_POINT_CREATION)
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Change> {
        self.changes.iter()
    }

    /// Moves cursors according to this change,
    /// Wont handle undo or redo
    pub fn move_cursors(&self, cursors: &mut [Cursor]) {
        for cursor in cursors {
            let mut range = cursor.selection().unwrap_or(cursor.pos()..cursor.pos());
            let removed: u64 = self
                .changes
                .iter()
                .take_while(|change| change.start() <= range.start)
                .map(|change| {
                    if change.end() < range.start {
                        change.range().len()
                    } else {
                        range.start - change.start()
                    }
                })
                .sum();

            let add: u64 = self
                .changes
                .iter()
                .take_while(|change| change.start() <= range.start)
                .map(|change| change.text().len() as u64)
                .sum();
            // log::debug!("+{add} -{removed}");

            let removed_post: u64 = self
                .changes
                .iter()
                .take_while(|change| change.start() <= range.end)
                // .map(|change| range.end - change.start())
                .map(|change| {
                    if change.end() < range.start {
                        change.range().len()
                    } else {
                        range.start - change.start()
                    }
                })
                .sum();

            let add_post: u64 = self
                .changes
                .iter()
                .take_while(|change| change.start() <= range.end)
                .map(|change| change.text().len() as u64)
                .sum();
            // log::debug!("post+{add_post} post-{removed_post}");

            range.start += add;
            range.start -= removed;
            range.end += add_post;
            range.end -= removed_post;

            // log::debug!("Cursor: {cursor:?} to {range:?}");
            cursor.to_range(&range);
            // log::debug!("Cursor: {cursor:?}");
        }
    }

    pub fn kind(&self) -> ChangesKind {
        if self.flags.contains(Flags::UNDO) {
            return ChangesKind::Undo;
        }

        if self.flags.contains(Flags::REDO) {
            return ChangesKind::Redo;
        }

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
            flags: Flags::default(),
        }
    }
}

impl From<Change> for Changes {
    fn from(value: Change) -> Self {
        Changes {
            changes: SortedVec::from(value),
            flags: Flags::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Change {
    // Range stored separately as Range<u64> does not implement ord
    /// Inclusive
    start: u64,
    /// Exclusive
    end: u64,
    text: Rc<Vec<u8>>,
}

impl Change {
    pub fn insert(at: u64, text: &[u8]) -> Change {
        Change {
            start: at,
            end: at,
            text: Rc::new(text.into()),
        }
    }

    fn insert_rc(at: u64, text: Rc<Vec<u8>>) -> Change {
        Change {
            start: at,
            end: at,
            text,
        }
    }

    pub fn remove(range: BufferRange) -> Change {
        Change {
            start: range.start,
            end: range.end,
            text: Rc::new(Vec::new()),
        }
    }

    pub fn replace(range: BufferRange, text: &[u8]) -> Change {
        Change {
            start: range.start,
            end: range.end,
            text: Rc::new(text.into()),
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
        const DISABLE_UNDO_POINT_CREATION = 0b00000100;
    }
}

#[derive(Debug, Clone)]
pub enum ChangesKind {
    Insert,
    Remove,
    Replace,
    Undo,
    Redo,
    Mixed,
}
