use std::{cmp, ops::Range};

#[derive(Debug)]
pub(crate) struct Selection {
    /// Position in buffer
    pos: usize,

    /// Visual column, keeps track of the wanted column for cursor, used if moving lines
    v_col: Option<usize>,

    /// Selection anchor. Selected range is formed from this position and the current `pos`
    anchor: Option<usize>,
}

impl Selection {
    pub fn new(pos: usize) -> Selection {
        Selection {
            pos,
            v_col: None,
            anchor: None,
        }
    }

    pub fn visual_column(&self) -> Option<usize> {
        self.v_col
    }

    pub fn goto(&mut self, pos: usize) {
        self.pos = pos;
        self.v_col = None;
    }

    pub fn goto_with_vcol(&mut self, pos: usize, v_col: usize) {
        self.pos = pos;
        self.v_col = Some(v_col);
    }

    pub fn start_selection(&mut self, anchor: usize) {
        self.anchor = Some(anchor);
    }

    pub fn unselect(&mut self) {
        self.anchor = None;
    }

    pub fn get(&self) -> Option<Range<usize>> {
        let anchor = self.anchor?;
        let min = cmp::min(self.pos, anchor);
        let max = cmp::max(self.pos, anchor);
        Some(min..max)
    }

    /// Check wether this position is in the selected area.
    pub fn contains(&self, pos: usize) -> bool {
        pos == self.pos || self.get().map_or(false, |range| range.contains(&pos))
    }
}
