use std::{cmp, mem};

use crate::{BufferRange, Range};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct Cursor {
    /// Position in buffer
    pos: u64,

    /// Selection anchor. Selected range is formed from this position and the current `pos`
    anchor: Option<u64>,

    /// keeps track of the wanted column for cursor, used if moving lines
    col: Option<usize>,
}

impl Cursor {
    pub fn new(pos: u64) -> Cursor {
        Cursor {
            pos,
            col: None,
            anchor: None,
        }
    }

    pub fn new_select(range: &BufferRange) -> Cursor {
        Cursor {
            pos: range.end,
            col: None,
            anchor: Some(range.start),
        }
    }

    pub fn pos(&self) -> u64 {
        self.pos
    }

    pub fn column(&self) -> Option<usize> {
        self.col
    }

    pub fn goto(&mut self, pos: u64) {
        self.col = None;
        self.goto_impl(pos);
    }

    pub fn goto_with_col(&mut self, pos: u64, col: usize) {
        self.col = Some(col);
        self.goto_impl(pos);
    }

    fn goto_impl(&mut self, pos: u64) {
        if let Some(ref mut anc) = self.anchor {
            if *anc == pos {
                *anc = self.pos;
            }
        }

        self.pos = pos;
    }

    pub fn set_column(&mut self, col: usize) {
        self.col = Some(col);
    }

    pub fn start_selection(&mut self) {
        self.anchor = Some(self.pos);
    }

    pub fn stop_selection(&mut self) {
        self.anchor = None;
    }

    pub fn select(&mut self, range: &BufferRange) {
        let is_select = self.anchor.is_some();
        if is_select {
            self.extend_to_include(range);
            self.contain_to(range);
        } else {
            self.anchor = Some(range.start);
            self.pos = range.end;
        }
    }

    pub fn selection(&self) -> Option<BufferRange> {
        let anc = self.anchor.as_ref()?;
        let min = self.pos.min(*anc);
        let max = self.pos.max(*anc);
        Some(Range::new(min, max))
    }

    pub fn start(&self) -> u64 {
        if let Some(anc) = self.anchor.as_ref() {
            std::cmp::min(self.pos, *anc)
        } else {
            self.pos
        }
    }

    pub fn end(&self) -> u64 {
        if let Some(anc) = self.anchor.as_ref() {
            std::cmp::max(self.pos, *anc)
        } else {
            self.pos
        }
    }

    pub fn take_selection(&mut self) -> Option<BufferRange> {
        let sel = self.selection()?;
        self.stop_selection();
        Some(sel)
    }

    pub fn contain_to(&mut self, range: &BufferRange) {
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

    pub fn to_range(&mut self, other: &BufferRange) {
        // Dont extend into a selection in not necessary
        if other.end - other.start <= 1 && self.anchor.is_none() {
            self.goto(other.start);
            return;
        }

        self.extend_to_include(other);
        self.contain_to(other);

        let unanchor = self
            .anchor
            .as_ref()
            .map(|anc| self.pos == *anc)
            .unwrap_or(false);
        if unanchor {
            self.stop_selection();
        }
    }

    pub fn extend_to_include_pos(&mut self, pos: u64) {
        let sel = self
            .selection()
            .unwrap_or(Range::new(self.pos(), self.pos() + 1));
        if sel.start <= pos && pos < sel.end {
            return;
        }

        if let Some(anc) = self.anchor.as_mut() {
            if *anc < self.pos {
                *anc = cmp::min(pos, *anc);
                self.pos = cmp::max(pos, self.pos);
            } else {
                *anc = cmp::max(pos, *anc);
                self.pos = cmp::min(pos, self.pos);
            }
            return;
        }

        let min = cmp::min(pos, self.pos);
        let max = cmp::max(pos, self.pos);
        self.pos = min;
        self.anchor = Some(max);
    }

    /// Extend this cursor to cover the specified range.
    /// If the range is a single value this will not convert the
    /// cursor into an selection.
    pub fn extend_to_include(&mut self, other: &BufferRange) {
        let sel = self
            .selection()
            .unwrap_or(Range::new(self.pos(), self.pos() + 1));
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

    pub fn is_selecting(&self) -> bool {
        self.anchor.is_some()
    }
}

impl Default for Cursor {
    fn default() -> Self {
        Cursor::new(0)
    }
}
