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

    pub fn anchor(&mut self) {
        self.anchor = Some(self.pos);
    }

    pub fn unanchor(&mut self) {
        self.anchor = None;
    }

    pub fn selection(&self) -> Option<Range<usize>> {
        let anchor = self.anchor?;
        let min = self.pos.min(anchor);
        let max = self.pos.max(anchor);
        Some(min..max)
    }

    pub fn take_selection(&mut self) -> Option<Range<usize>> {
        let sel = self.selection()?;
        self.unanchor();
        Some(sel)
    }

    /// Extend this cursor to cover the specified range.
    /// If the range is a single value this will not convert the
    /// cursor into an selection.
    pub fn extend_to_include(&mut self, other: &Range<usize>) {
        let sel = self.selection().unwrap_or(self.pos()..self.pos() + 1);
        if sel.includes(other) {
            return;
        }

        if self.anchor.is_some() {
            if let Some(anc) = self.anchor.as_mut() {
                let min = if *anc < self.pos { anc } else { &mut self.pos };
                *min = other.start;
            }

            if let Some(anc) = self.anchor.as_mut() {
                let max = if *anc < self.pos { &mut self.pos } else { anc };
                *max = other.end;
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

    // /// Remove the selected text from the buffer and restore cursor to non
    // /// selecting.
    // pub fn remove_selection(&mut self, buf: &mut Buffer) {
    //     if let Some(sel) = self.selection() {
    //         let Range { start, .. } = sel;
    //         buf.remove(sel);
    //         self.unanchor();
    //         self.goto(start);
    //     }
    // }
}

impl Default for Cursor {
    fn default() -> Self {
        Cursor::new(0)
    }
}
