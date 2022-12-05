use std::{cmp, ops::Range};

#[derive(Debug)]
pub(crate) struct Cursor {
    /// Position in buffer
    pos: usize,

    /// Visual column, keeps track of the wanted column for cursor, used if moving lines
    v_col: Option<usize>,

    /// Selection anchor. Selected range is formed from this position and the current `pos`
    anchor: Option<usize>,
}

impl Cursor {
    pub fn new(pos: usize) -> Cursor {
        Cursor {
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

    pub fn anchor(&mut self) {
        self.anchor = Some(self.pos);
    }

    pub fn disanchor(&mut self) {
        self.anchor = None;
    }

    pub fn selection(&self) -> Option<Range<usize>> {
        let anchor = self.anchor?;
        let min = cmp::min(self.pos, anchor);
        let max = cmp::max(self.pos, anchor);
        Some(min..max)
    }
}
