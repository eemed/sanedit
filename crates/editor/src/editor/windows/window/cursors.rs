mod cursor;

use std::{cmp::min, ops::Range};

pub(crate) use cursor::Cursor;

use crate::{common::range::RangeUtils, editor::buffers::SortedRanges};

#[derive(Debug, Clone)]
pub(crate) struct Cursors {
    /// Sorted list of cursors based on their positions
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
        let pos = self
            .cursors
            .binary_search_by(|c| c.pos().cmp(&cursor.pos()))
            .unwrap_or_else(|n| n);
        self.cursors.insert(pos, cursor);
    }

    pub fn push_primary(&mut self, cursor: Cursor) {
        let pos = self
            .cursors
            .binary_search_by(|c| c.pos().cmp(&cursor.pos()))
            .unwrap_or_else(|n| n);
        self.cursors.insert(pos, cursor);
        self.primary = pos;
    }

    /// Remove cursor at position pos
    pub fn remove(&mut self, _pos: usize) {
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

        self.cursors.sort();

        for i in (1..self.cursors.len()).rev() {
            let cur = {
                let cursor = &self.cursors[i - 1];
                cursor.selection().unwrap_or(cursor.pos()..cursor.pos() + 1)
            };

            let next = {
                let cursor = &self.cursors[i];
                cursor.selection().unwrap_or(cursor.pos()..cursor.pos() + 1)
            };

            if cur.overlaps(&next) {
                self.cursors.remove(i);
                let cur = &mut self.cursors[i - 1];
                cur.extend_to_include(&next);
            }
        }

        self.primary = min(self.primary, self.cursors.len() - 1);
    }

    pub fn shrink_cursor_to_range(&mut self, range: Range<usize>) {
        for cursor in &mut self.cursors {
            cursor.shrink_to_range(&range)
        }
    }

    pub fn swap_selection_dir(&mut self) {
        for cur in &mut self.cursors {
            cur.swap_selection_dir();
        }
    }

    pub fn primary_next(&mut self) {
        if self.primary + 1 < self.cursors.len() {
            self.primary += 1;
        } else {
            self.primary = 0;
        }
    }

    pub fn primary_prev(&mut self) {
        if self.primary == 0 {
            // Wrap to end
            self.primary = self.cursors.len() - 1;
        } else {
            self.primary -= 1;
        }
    }

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

impl From<&Cursors> for Vec<usize> {
    fn from(cursors: &Cursors) -> Self {
        let positions: Vec<usize> = cursors.cursors().iter().map(|c| c.pos()).collect();
        positions.into()
    }
}

impl From<&Cursors> for SortedRanges {
    /// Crate Sorted ranges from all of the cursors selections
    fn from(cursors: &Cursors) -> Self {
        let mut selections = vec![];

        for cursor in cursors.cursors() {
            if let Some(sel) = cursor.selection() {
                selections.push(sel);
            }
        }

        selections.into()
    }
}
