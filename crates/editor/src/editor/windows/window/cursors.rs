use std::cmp::min;

use rustc_hash::FxHashSet;
use sanedit_buffer::Mark;
use sanedit_core::{BufferRange, Cursor};
use sanedit_utils::ranges::OverlappingRanges;

use crate::editor::buffers::Buffer;

#[derive(Debug, Clone)]
pub(crate) struct Cursors {
    // I would like this to be sorted but sortedvec ensures nothing because
    // cursors move all the time. It should be sorted/checked after every
    // change. So its unsorted for now.
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

    pub fn primary_mut(&mut self) -> &mut Cursor {
        &mut self.cursors[self.primary]
    }

    pub fn start_selection(&mut self) {
        for cursor in &mut self.cursors {
            cursor.start_selection();
        }
    }

    pub fn stop_selection(&mut self) {
        for cursor in &mut self.cursors {
            cursor.stop_selection();
        }
    }

    pub fn cursors(&self) -> &[Cursor] {
        &self.cursors
    }

    pub fn cursors_mut(&mut self) -> &mut [Cursor] {
        &mut self.cursors
    }

    pub fn trim_whitespace(&mut self, buf: &Buffer) -> bool {
        let mut last_pos = 0;
        let mut changed = false;
        self.cursors.retain_mut(|cursor| {
            let Some(mut sel) = cursor.selection() else {
                return true;
            };
            let slice = buf.slice(sel.clone());
            let mut chars = slice.chars();
            let mut left = 0;
            while let Some((start, end, ch)) = chars.next() {
                if ch.is_whitespace() {
                    left += end - start;
                } else {
                    break;
                }
            }

            let mut chars_rev = slice.chars_at(slice.len());
            let mut right = 0;
            while let Some((start, end, ch)) = chars_rev.prev() {
                if ch.is_whitespace() {
                    right += end - start;
                } else {
                    break;
                }
            }

            if left + right >= slice.len() {
                last_pos = sel.start;
                false
            } else {
                changed = true;
                sel.start += left;
                sel.end -= right;
                cursor.select(&sel);
                true
            }
        });

        if self.cursors.is_empty() {
            self.cursors.push(Cursor::new(last_pos));
        }

        self.primary = min(self.primary, self.cursors.len() - 1);
        changed
    }

    /// Add a new cursor
    pub fn push(&mut self, cursor: Cursor) {
        self.cursors.push(cursor);
    }

    /// Push a new primary cursor
    pub fn push_primary(&mut self, cursor: Cursor) {
        let len = self.cursors.len();
        self.push(cursor);
        self.primary = len;
    }

    pub fn replace_primary(&mut self, cursor: Cursor) {
        let primary = self.cursors.get_mut(self.primary).unwrap();
        *primary = cursor;
    }

    /// Remove all cursors except the primary one
    pub fn remove_except_primary(&mut self) {
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

        let mut singles = FxHashSet::default();
        let mut selections = OverlappingRanges::default();
        for cursor in &self.cursors {
            match cursor.selection() {
                Some(range) => {
                    selections.add(range.into());
                }
                _ => {
                    singles.insert(cursor.pos());
                }
            }
        }

        // Handle ranges
        for range in selections.iter() {
            let range: BufferRange = range.into();
            let mut i = 0;
            while i < self.cursors.len() {
                let cursor = &mut self.cursors[i];
                match cursor.selection() {
                    Some(crange) => {
                        if cursor.start() == range.start {
                            cursor.select(&range);
                            i += 1;
                        } else if range.includes(&crange) {
                            // remove if contained in another range
                            self.cursors.remove(i);
                        } else {
                            i += 1;
                        }
                    }
                    None => {
                        // Just remove single cursors that are in ranges
                        let cp = cursor.pos();
                        if range.contains(&cp) {
                            self.cursors.remove(i);
                            singles.remove(&cp);
                        } else {
                            i += 1;
                        }
                    }
                }
            }
        }

        // Handle single cursors
        self.cursors.retain(|cursor| {
            let cp = cursor.pos();
            let keep = singles.contains(&cp) || cursor.selection().is_some();
            if keep {
                singles.remove(&cp);
            }

            keep
        });

        self.primary = min(self.primary, self.cursors.len() - 1);
    }

    /// Make sure all cursors are contained in range
    /// Moves / shrinks cursors if needed
    pub fn contain_to(&mut self, range: BufferRange) {
        for cursor in &mut self.cursors {
            cursor.contain_to(&range)
        }
    }

    pub fn swap_selection_dir(&mut self) {
        for cur in &mut self.cursors {
            cur.swap_selection_dir();
        }
    }

    /// Selects the next cursor in terms of position
    pub fn primary_next(&mut self) {
        if self.cursors.len() < 2 {
            return;
        }

        let pos = self.cursors[self.primary].pos();
        let mut n = self.primary;
        let mut next = u64::MAX;

        for (i, cursor) in self.cursors.iter().enumerate() {
            let cp = cursor.pos();

            // Next cursor with smallest amount of distance to current
            if i != self.primary && cp > pos && next - pos > cp - pos {
                next = cp;
                n = i;
            }
        }

        if n == self.primary {
            // Take smallest
            for (i, cursor) in self.cursors.iter().enumerate() {
                let cp = cursor.pos();
                if next > cp {
                    next = cp;
                    n = i;
                }
            }
        }

        self.primary = n;
    }

    /// Selects the previous cursor in terms of position
    pub fn primary_prev(&mut self) {
        if self.cursors.len() < 2 {
            return;
        }

        let pos = self.cursors[self.primary].pos();
        let mut n = self.primary;
        let mut prev = 0;

        for (i, cursor) in self.cursors.iter().enumerate() {
            let cp = cursor.pos();

            // prev cursor with smallest amount of distance to current
            if i != self.primary && cp < pos && pos - prev > pos - cp {
                prev = cp;
                n = i;
            }
        }

        if n == self.primary {
            // Take largest
            for (i, cursor) in self.cursors.iter().enumerate() {
                let cp = cursor.pos();
                if prev < cp {
                    prev = cp;
                    n = i;
                }
            }
        }

        self.primary = n;
    }

    /// Remove primary cursor if more cursors exist
    pub fn remove_primary(&mut self) {
        if self.cursors.len() < 2 {
            return;
        }

        let old = self.primary;
        self.primary_next();
        self.cursors.remove(old);
        if self.primary > old {
            self.primary -= 1;
        }
    }

    pub fn iter(&self) -> std::slice::Iter<Cursor> {
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
