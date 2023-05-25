mod cursor;

use std::ops::Range;

pub(crate) use cursor::Cursor;

use crate::common::range::RangeUtils;

#[derive(Debug)]
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
    pub fn remove(&mut self, pos: usize) {}

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

        let as_ranges: Vec<Range<usize>> = self
            .cursors
            .iter()
            .map(|c| c.selection().unwrap_or(c.pos()..c.pos() + 1))
            .collect();

        let mut delete = vec![];
        for i in 1..as_ranges.len() {
            let cur = &as_ranges[i];
            let prev = &as_ranges[i - 1];

            if cur.overlaps(prev) {
                delete.push(i);
                let prev = &mut self.cursors[i - 1];
                prev.extend_to_include(cur);
            }
        }

        delete.into_iter().rev().for_each(|i| {
            self.cursors.remove(i);
        });
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
