#[cfg(test)]
mod test;

use std::{borrow::Cow, rc::Rc};

use sanedit_buffer::{utf8::EndOfLine, PieceTree, PieceTreeView};
use sanedit_utils::sorted_vec::SortedVec;

use crate::{range::BufferRange, Cursor, Range};

use self::flags::Flags;

#[derive(Debug)]
pub struct Edit {
    pub buf: PieceTreeView,
    pub changes: Changes,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

    /// Jump to a undo index
    pub fn undo_jump(index: usize) -> Changes {
        let index = index as u64;
        Changes {
            changes: SortedVec::from(Change::remove(Range::new(index, index))),
            flags: flags::UNDO_JUMP,
        }
    }

    pub fn undo() -> Changes {
        Changes {
            changes: SortedVec::new(),
            flags: flags::UNDO,
        }
    }

    pub fn redo() -> Changes {
        Changes {
            changes: SortedVec::new(),
            flags: flags::REDO,
        }
    }

    pub fn is_undo(&self) -> bool {
        self.flags & flags::UNDO == flags::UNDO
    }

    pub fn is_undo_jump(&self) -> bool {
        self.flags & flags::UNDO_JUMP == flags::UNDO_JUMP
    }

    pub fn is_redo(&self) -> bool {
        self.flags & flags::REDO == flags::REDO
    }

    pub fn undo_jump_index(&self) -> usize {
        self.changes[0].range.start as usize
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

            if !range.is_empty() {
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
            pt.remove(range);
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
        self.flags |= flags::DISABLE_UNDO_POINT_CREATION;
    }

    pub fn allows_undo_point_creation(&self) -> bool {
        self.flags & flags::DISABLE_UNDO_POINT_CREATION == 0
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Change> {
        self.changes.iter()
    }

    fn added(&self, pos: u64, inclusive: bool) -> u64 {
        self.changes
            .iter()
            .take_while(|change| {
                if inclusive {
                    change.start() <= pos
                } else {
                    change.start() < pos
                }
            })
            .map(|change| {
                if pos == change.start() && change.cursor_offset.is_some() {
                    change.cursor_offset.unwrap()
                } else {
                    change.text().len() as u64
                }
            })
            .sum()
    }

    fn removed(&self, pos: u64, inclusive: bool) -> u64 {
        self.changes
            .iter()
            .take_while(|change| {
                if inclusive {
                    change.start() <= pos
                } else {
                    change.start() < pos
                }
            })
            .map(|change| {
                if change.end() < pos || pos < change.start() {
                    change.range().len()
                } else {
                    pos - change.start()
                }
            })
            .sum()
    }

    fn is_removed(&self, range: &BufferRange) -> bool {
        self.changes
            .iter()
            .any(|change| change.range.includes(range))
    }

    fn is_replaced(&self, range: &BufferRange) -> Option<&Change> {
        self.changes.iter().find(|change| &change.range == range)
    }

    pub fn keep_cursors_still(&self, cursors: &mut [Cursor]) {
        for cursor in cursors {
            match cursor.selection() {
                Some(range) => {
                    if self.is_removed(&range) {
                        let mut pos = range.start;
                        pos -= self.removed(range.start, true);

                        // Stop selection is completely removed
                        cursor.stop_selection();
                        cursor.goto(pos);
                    } else {
                        let mut start = range.start;
                        let mut end = range.end;
                        start -= self.removed(range.start, true);
                        end -= self.removed(range.end, false);

                        cursor.to_range(&Range::new(start, end));
                    }
                }
                None => {
                    let mut npos = cursor.pos();

                    npos -= self.removed(cursor.pos(), true);

                    cursor.goto(npos);
                }
            }

            // log::debug!("Cursor: {cursor:?}");
        }
    }

    /// Moves cursors according to this change,
    /// Wont handle undo or redo
    pub fn move_cursors(&self, cursors: &mut [Cursor], reselect_replacement: bool) {
        for cursor in cursors {
            match cursor.selection() {
                Some(range) => {
                    if reselect_replacement {
                        if let Some(change) = self.is_replaced(&range) {
                            let mut pos = range.start;
                            pos += self.added(range.start, true);
                            pos -= self.removed(range.start, true);

                            let nsel = Range::new(pos - change.text().len() as u64, pos);
                            cursor.select(&nsel);

                            continue;
                        }
                    }

                    if self.is_removed(&range) {
                        let mut pos = range.start;
                        pos += self.added(range.start, true);
                        pos -= self.removed(range.start, true);

                        // Stop selection is completely removed
                        cursor.stop_selection();
                        cursor.goto(pos);
                    } else {
                        let mut start = range.start;
                        let mut end = range.end;
                        start += self.added(range.start, true);
                        start -= self.removed(range.start, true);
                        end += self.added(range.end, false);
                        end -= self.removed(range.end, false);

                        cursor.to_range(&Range::new(start, end));
                    }
                }
                None => {
                    let mut npos = cursor.pos();

                    npos += self.added(cursor.pos(), true);
                    npos -= self.removed(cursor.pos(), true);

                    cursor.goto(npos);
                }
            }

            // log::debug!("Cursor: {cursor:?}");
        }
    }

    pub fn move_offset(&self, offset: u64) -> u64 {
        let mut npos = offset;
        npos += self.added(offset, true);
        npos -= self.removed(offset, true);
        npos
    }

    pub fn kind(&self) -> ChangesKind {
        if self.flags & flags::UNDO != 0 {
            return ChangesKind::Undo;
        }

        if self.flags & flags::REDO != 0 {
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

    pub fn has_insert_eol(&self) -> bool {
        self.changes.iter().all(Change::has_eol)
    }

    fn after_ranges(&self) -> Vec<BufferRange> {
        let mut ranges = Vec::with_capacity(self.changes.len());

        for change in self.changes.iter() {
            let start = change.start();
            let mut end = change.end();

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
            ranges.push(Range::new(start, end));
        }

        ranges
    }

    fn before_ranges(&self) -> Vec<BufferRange> {
        let mut ranges = Vec::with_capacity(self.changes.len());

        for change in self.changes.iter() {
            ranges.push(change.range());
        }

        ranges
    }

    pub fn needs_undo_point(&self, previous: Option<&Changes>) -> bool {
        // no previous edits, undo point should be created
        if previous.is_none() {
            return true;
        }

        use ChangesKind::*;

        let pchange = previous.unwrap();
        match (pchange.kind(), self.kind()) {
            (Insert, Insert) => {
                if self.has_insert_eol() {
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
    range: Range<u64>,
    text: Rc<Vec<u8>>,

    /// By default cursor is placed at the start/end of a change.
    /// If we want cursor to be set in the middle, this offset can be used.
    pub cursor_offset: Option<u64>,
}

impl Change {
    pub fn insert(at: u64, text: &[u8]) -> Change {
        Change {
            range: Range::new(at, at),
            text: Rc::new(text.into()),
            cursor_offset: None,
        }
    }

    fn insert_rc(at: u64, text: Rc<Vec<u8>>) -> Change {
        Change {
            range: Range::new(at, at),
            text,
            cursor_offset: None,
        }
    }

    pub fn remove(range: BufferRange) -> Change {
        Change {
            range,
            text: Rc::new(Vec::new()),
            cursor_offset: None,
        }
    }

    pub fn replace(range: BufferRange, text: &[u8]) -> Change {
        Change {
            range,
            text: Rc::new(text.into()),
            cursor_offset: None,
        }
    }

    pub fn range(&self) -> BufferRange {
        self.range.clone()
    }

    pub fn start(&self) -> u64 {
        self.range.start
    }

    pub fn end(&self) -> u64 {
        self.range.end
    }

    pub fn text(&self) -> &[u8] {
        &self.text
    }

    pub fn is_remove(&self) -> bool {
        self.text.is_empty() && !self.range.is_empty()
    }

    pub fn is_insert(&self) -> bool {
        !self.text.is_empty() && self.range.is_empty()
    }

    pub fn is_replace(&self) -> bool {
        !self.text.is_empty() && !self.range.is_empty()
    }

    pub fn has_eol(&self) -> bool {
        EndOfLine::has_eol(self.text.as_ref())
    }
}

mod flags {
    pub(crate) type Flags = u8;
    pub(crate) const UNDO: u8 = 1 << 0;
    pub(crate) const REDO: u8 = 1 << 1;
    pub(crate) const DISABLE_UNDO_POINT_CREATION: u8 = 1 << 2;
    pub(crate) const UNDO_JUMP: u8 = UNDO | 1 << 3;
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
