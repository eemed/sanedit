use std::{cmp::min, ops::RangeBounds};

use sanedit_buffer::Mark;
use sanedit_core::{BufferRange, Cursor};

use crate::{
    common::text::{trim_whitespace, trim_whitespace_back},
    editor::buffers::Buffer,
};

/// Guard for cursors to ensure it is sorted and non overlapping after mutation
#[derive(Debug)]
pub(crate) struct CursorsGuard<'a> {
    cursors: &'a mut Cursors,
}

impl<'a> CursorsGuard<'a> {
    pub fn primary(&mut self) -> &mut Cursor {
        &mut self.cursors.cursors[self.cursors.primary]
    }

    /// Add a new cursor
    pub fn push(&mut self, cursor: Cursor) {
        self.cursors.cursors.push(cursor);
    }

    /// Push a new primary cursor
    pub fn push_primary(&mut self, cursor: Cursor) {
        let pos = self.cursors.len();
        self.cursors.cursors.push(cursor);
        self.cursors.primary = pos;
    }

    pub fn replace_primary(&mut self, cursor: Cursor) {
        let primary = &mut self.cursors.cursors[self.cursors.primary];
        *primary = cursor;
    }

    /// Remove primary cursor if more cursors exist
    pub fn remove_primary(&mut self) {
        if self.cursors.len() < 2 {
            return;
        }

        let old = self.cursors.primary;
        self.cursors.primary_next();
        self.cursors.cursors.remove(old);
        if self.cursors.primary > old {
            self.cursors.primary -= 1;
        }
    }

    /// Remove all cursors except the primary one
    pub fn remove_except_primary(&mut self) {
        let cursor = self.cursors.cursors.swap_remove(self.cursors.primary);
        self.cursors.cursors.clear();
        self.cursors.cursors.push(cursor);
        self.cursors.primary = 0;
    }
}

impl<'a> std::ops::Deref for CursorsGuard<'a> {
    type Target = [Cursor];

    fn deref(&self) -> &Self::Target {
        &self.cursors.cursors
    }
}

impl<'a> std::ops::DerefMut for CursorsGuard<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.cursors.cursors
    }
}

impl<'a> Drop for CursorsGuard<'a> {
    fn drop(&mut self) {
        self.cursors.sort_and_merge_overlapping();
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Cursors {
    /// Non overlapping, sorted set of cursors.
    cursors: Vec<Cursor>,
    primary: usize,
}

impl Cursors {
    pub fn new(cursor: Cursor) -> Cursors {
        Cursors {
            cursors: vec![cursor],
            primary: 0,
        }
    }

    pub fn primary_index(&self) -> usize {
        self.primary
    }

    pub fn len(&self) -> usize {
        self.cursors.len()
    }

    pub fn primary(&self) -> &Cursor {
        &self.cursors[self.primary]
    }

    pub fn start_selection(&mut self) {
        for cursor in &mut self.cursors {
            cursor.start_selection();
        }
    }

    pub fn cursors(&self) -> &[Cursor] {
        &self.cursors
    }

    pub fn cursors_mut<'a>(&'a mut self) -> CursorsGuard<'a> {
        CursorsGuard { cursors: self }
    }

    pub fn trim_whitespace(&mut self, buf: &Buffer) -> bool {
        let mut last_pos = 0;
        let mut changed = false;
        self.cursors.retain_mut(|cursor| {
            let Some(sel) = cursor.selection() else {
                return true;
            };
            let slice = buf.slice(sel);
            let slice = trim_whitespace(&slice);
            let slice = trim_whitespace_back(&slice);

            let srange = slice.range();
            if sel.start == srange.start && sel.end == srange.end {
                last_pos = sel.start;
                false
            } else {
                changed = true;
                cursor.select(slice.range());
                true
            }
        });

        if self.cursors.is_empty() {
            self.cursors.push(Cursor::new(last_pos));
        }

        self.primary = min(self.primary, self.cursors.len() - 1);
        changed
    }

    fn sort(&mut self) {
        let primary = self.cursors.swap_remove(self.primary);
        self.cursors.sort();
        let pos = match self.cursors.binary_search(&primary) {
            Ok(n) => n,
            Err(n) => n,
        };
        self.cursors.insert(pos, primary);
        self.primary = pos;
    }

    fn merge_overlapping(&mut self) {
        let mut merged = Vec::with_capacity(self.cursors.len());

        for cursor in std::mem::take(&mut self.cursors) {
            if merged.is_empty() {
                merged.push(cursor);
                continue;
            }

            let last = merged.last_mut().unwrap();
            if last.end() < cursor.start() {
                merged.push(cursor);
            } else {
                let end = std::cmp::max(last.end(), cursor.end());
                last.extend_to_include_pos(end);
            }
        }

        self.cursors = merged;
        self.primary = min(self.primary, self.cursors.len() - 1);
    }

    /// Merge overlapping cursors into one
    pub fn sort_and_merge_overlapping(&mut self) {
        if self.cursors.len() <= 1 {
            return;
        }

        self.sort();
        self.merge_overlapping();
    }

    /// Make sure all cursors are contained in range
    /// Moves / shrinks cursors if needed
    pub fn contain_to<R: RangeBounds<u64>>(&mut self, range: R) {
        let range = BufferRange::from_bounds(range);
        for cursor in &mut self.cursors {
            cursor.contain_to(range)
        }

        self.sort_and_merge_overlapping();
    }

    pub fn swap_selection_dir(&mut self) {
        for cur in &mut self.cursors {
            cur.swap_selection_dir();
        }
    }

    /// Selects the next cursor in terms of position
    pub fn primary_next(&mut self) {
        self.primary += 1;

        if self.primary >= self.cursors.len() {
            self.primary = 0;
        }
    }

    /// Selects the previous cursor in terms of position
    pub fn primary_prev(&mut self) {
        if self.primary == 0 {
            self.primary = self.cursors.len().saturating_sub(1);
        } else {
            self.primary -= 1;
        }
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Cursor> {
        self.cursors.iter()
    }

    pub fn has_selections(&self) -> bool {
        self.cursors.iter().any(|c| c.selection().is_some())
    }

    pub fn mark_first(&self, buf: &Buffer) -> Mark {
        let pos = self
            .cursors()
            .iter()
            .map(Cursor::start)
            .min()
            .expect("No cursors found");
        buf.mark(pos)
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

impl From<Vec<Cursor>> for Cursors {
    fn from(value: Vec<Cursor>) -> Self {
        assert!(!value.is_empty(), "Empty cursor vector");

        let last = value.len().saturating_sub(1);
        Cursors {
            cursors: value,
            primary: last,
        }
    }
}

impl From<&Cursors> for Vec<u64> {
    fn from(cursors: &Cursors) -> Self {
        let positions: Vec<u64> = cursors.cursors().iter().map(|c| c.pos()).collect();
        positions
    }
}

impl From<&Cursors> for Vec<BufferRange> {
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
