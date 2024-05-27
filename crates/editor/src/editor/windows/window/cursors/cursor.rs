use std::{cmp, mem, ops::Range};

use crate::common::range::RangeUtils;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub(crate) struct Cursor {
    /// Position in buffer
    pos: usize,

    /// keeps track of the wanted column for cursor, used if moving lines
    col: Option<usize>,

    /// Selection anchor. Selected range is formed from this position and the current `pos`
    anchor: Option<usize>,
}

impl Cursor {
    pub fn new(pos: usize) -> Cursor {
        Cursor {
            pos,
            col: None,
            anchor: None,
        }
    }

    pub fn new_select(range: &Range<usize>) -> Cursor {
        Cursor {
            pos: range.end,
            col: None,
            anchor: Some(range.start),
        }
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn column(&self) -> Option<usize> {
        self.col
    }

    pub fn goto(&mut self, pos: usize) {
        self.pos = pos;
        self.col = None;
    }

    pub fn goto_with_col(&mut self, pos: usize, col: usize) {
        self.pos = pos;
        self.col = Some(col);
    }

    pub fn set_column(&mut self, col: usize) {
        self.col = Some(col);
    }

    pub fn anchor(&mut self) {
        self.anchor = Some(self.pos);
    }

    pub fn unanchor(&mut self) {
        self.anchor = None;
    }

    pub fn select(&mut self, range: Range<usize>) {
        self.anchor = Some(range.start);
        self.pos = range.end;
    }

    pub fn selection(&self) -> Option<Range<usize>> {
        let anc = self.anchor.as_ref()?;
        let min = self.pos.min(*anc);
        let max = self.pos.max(*anc);
        Some(min..max)
    }

    pub fn take_selection(&mut self) -> Option<Range<usize>> {
        let sel = self.selection()?;
        self.unanchor();
        Some(sel)
    }

    pub fn shrink_to_range(&mut self, range: &Range<usize>) {
        if self.pos > range.end {
            self.pos = range.end
        }

        if self.pos < range.start {
            self.pos = range.start
        }

        if let Some(ref mut anc) = self.anchor {
            if *anc > range.end {
                *anc = range.end;
            }

            if *anc < range.start {
                *anc = range.start;
            }
        }
    }

    pub fn to_range(&mut self, other: &Range<usize>) {
        // Dont extend into a selection in not necessary
        if other.len() == 1 && self.anchor.is_none() {
            self.goto(other.start);
            return;
        }

        self.extend_to_include(other);
        self.shrink_to_range(other);
    }

    /// Extend this cursor to cover the specified range.
    /// If the range is a single value this will not convert the
    /// cursor into an selection.
    pub fn extend_to_include(&mut self, other: &Range<usize>) {
        let sel = self.selection().unwrap_or(self.pos()..self.pos() + 1);
        if sel.includes(other) {
            return;
        }

        if let Some(anc) = self.anchor.as_mut() {
            if *anc < self.pos {
                *anc = other.start;
                self.pos = other.end;
            } else {
                *anc = other.end;
                self.pos = other.start;
            }
            return;
        }

        let min = cmp::min(other.start, self.pos);
        let max = cmp::max(other.end, self.pos);
        self.pos = min;
        self.anchor = Some(max);
    }

    pub fn swap_selection_dir(&mut self) {
        if let Some(anc) = &mut self.anchor {
            mem::swap(anc, &mut self.pos);
        }
    }
}

impl Default for Cursor {
    fn default() -> Self {
        Cursor::new(0)
    }
}
