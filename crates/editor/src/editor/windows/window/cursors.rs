use std::{cmp::min, ops::Range};

use sanedit_core::{BufferRangeExt as _, Cursor, RangeUtils as _};
use sanedit_utils::ranges::OverlappingRanges;

#[derive(Debug, Clone)]
pub struct Cursors {
    cursors: Vec<Cursor>,
    primary: usize,
}

impl Cursors {
    pub fn len(&self) -> usize {
        self.cursors.len()
    }

    pub fn primary(&self) -> &Cursor {
        &self.cursors[self.primary]
    }

    pub fn primary_mut(&mut self) -> &mut Cursor {
        &mut self.cursors[self.primary]
    }

    pub fn start_selection(&mut self) {
        for cursor in &mut self.cursors {
            cursor.anchor();
        }
    }

    pub fn cursors(&self) -> &[Cursor] {
        &self.cursors
    }

    pub fn cursors_mut(&mut self) -> &mut [Cursor] {
        &mut self.cursors
    }

    /// Add a new cursor
    pub fn push(&mut self, cursor: Cursor) {
        self.cursors.push(cursor);
    }

    pub fn push_primary(&mut self, cursor: Cursor) {
        let len = self.cursors.len();
        self.push(cursor);
        self.primary = len;
    }

    /// Remove cursor at position pos
    pub fn remove(&mut self, _pos: u64) {
        todo!()
    }

    /// Remove all cursors except the primary one
    pub fn remove_secondary_cursors(&mut self) {
        let cursor = self.cursors.swap_remove(self.primary);
        self.cursors.clear();
        self.cursors.push(cursor);
        self.primary = 0;
    }

    /// Merge overlapping cursors into one
    pub fn merge_overlapping(&mut self) {
        if self.cursors.len() <= 1 {
            return;
        }

        let mut ranges = OverlappingRanges::default();
        for cursor in &self.cursors {
            let range = cursor.selection().unwrap_or(cursor.pos()..cursor.pos() + 1);
            ranges.add(range);
        }

        for range in ranges.iter() {
            let mut i = 0;
            while i < self.cursors.len() {
                let cursor = &mut self.cursors[i];
                let crange = cursor.selection().unwrap_or(cursor.pos()..cursor.pos() + 1);
                if cursor.start() == range.start {
                    if range.len() > 1 {
                        cursor.select(range.clone());
                    }

                    i += 1;
                } else if range.includes(&crange) {
                    self.cursors.remove(i);
                } else {
                    i += 1;
                }
            }
        }

        self.primary = min(self.primary, self.cursors.len() - 1);
    }

    pub fn shrink_cursor_to_range(&mut self, range: Range<u64>) {
        for cursor in &mut self.cursors {
            cursor.shrink_to_range(&range)
        }
    }

    pub fn swap_selection_dir(&mut self) {
        for cur in &mut self.cursors {
            cur.swap_selection_dir();
        }
    }

    // TODO
    pub fn primary_next(&mut self) {
        if self.primary + 1 < self.cursors.len() {
            self.primary += 1;
        } else {
            self.primary = 0;
        }
    }

    // TODO
    pub fn primary_prev(&mut self) {
        if self.primary == 0 {
            // Wrap to end
            self.primary = self.cursors.len() - 1;
        } else {
            self.primary -= 1;
        }
    }

    // TODO
    pub fn remove_primary(&mut self) {
        if self.cursors.len() < 2 {
            return;
        }

        self.cursors.remove(self.primary);
        // Wrap to start
        if self.primary >= self.cursors.len() {
            self.primary = 0;
        }
    }

    pub fn iter(&self) -> std::slice::Iter<Cursor> {
        self.cursors.iter()
    }

    pub fn has_selections(&self) -> bool {
        self.cursors.iter().any(|c| c.selection().is_some())
    }
}

impl Default for Cursors {
    fn default() -> Self {
        Cursors {
            cursors: vec![Cursor::default()],
            primary: 0,
        }
    }
}

impl From<&Cursors> for Vec<u64> {
    fn from(cursors: &Cursors) -> Self {
        let positions: Vec<u64> = cursors.cursors().iter().map(|c| c.pos()).collect();
        positions.into()
    }
}

impl From<&Cursors> for Vec<Range<u64>> {
    /// Crate Sorted ranges from all of the cursors selections
    fn from(cursors: &Cursors) -> Self {
        let mut selections = vec![];

        for cursor in cursors.cursors() {
            if let Some(sel) = cursor.selection() {
                selections.push(sel);
            }
        }

        selections
    }
}
