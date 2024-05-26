use std::{cmp, mem, ops::Range};

use crate::common::range::RangeUtils;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
enum Anchor {
    Range(usize, usize),
    Position(usize),
}

impl Anchor {
    pub fn min(&mut self) -> &mut usize {
        match self {
            Anchor::Range(a, _) => a,
            Anchor::Position(p) => p,
        }
    }

    pub fn max(&mut self) -> &mut usize {
        match self {
            Anchor::Range(_, b) => b,
            Anchor::Position(p) => p,
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub(crate) struct Cursor {
    /// Position in buffer
    pos: usize,

    /// keeps track of the wanted column for cursor, used if moving lines
    col: Option<usize>,

    /// Selection anchor. Selected range is formed from this position and the current `pos`
    anchor: Option<Anchor>,
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
            anchor: Some(Anchor::Position(range.start)),
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

    pub fn anchor_range(&mut self, range: Range<usize>) {
        let Range { start, end } = range;
        self.anchor = Some(Anchor::Range(start, end));
    }

    pub fn anchor(&mut self) {
        self.anchor = Some(Anchor::Position(self.pos));
    }

    pub fn unanchor(&mut self) {
        self.anchor = None;
    }

    pub fn select(&mut self, range: Range<usize>) {
        self.anchor = Some(Anchor::Position(range.start));
        self.pos = range.end;
    }

    pub fn selection(&self) -> Option<Range<usize>> {
        match self.anchor.as_ref()? {
            Anchor::Range(s, e) => {
                if self.pos < *s {
                    Some(self.pos..*e)
                } else if *e < self.pos {
                    Some(*s..self.pos)
                } else {
                    Some(*s..*e)
                }
            }
            Anchor::Position(anc) => {
                let min = self.pos.min(*anc);
                let max = self.pos.max(*anc);
                Some(min..max)
            }
        }
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
            let max = anc.max();
            if *max > range.end {
                *max = range.end;
            }

            let min = anc.min();
            if *min < range.start {
                *min = range.start;
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
            let amin = anc.min();
            let min = if *amin < self.pos {
                amin
            } else {
                &mut self.pos
            };
            *min = other.start;

            let amax = anc.max();
            let max = if *amax < self.pos {
                &mut self.pos
            } else {
                amax
            };
            *max = other.end;
            return;
        }

        let min = cmp::min(other.start, self.pos);
        let max = cmp::max(other.end, self.pos);
        self.pos = min;
        self.anchor = Some(Anchor::Position(max));
    }

    pub fn swap_selection_dir(&mut self) {
        todo!()
        // if let Some(anc) = &mut self.anchor {
        //     mem::swap(anc, &mut self.pos);
        // }
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
