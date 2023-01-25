use std::{cmp, ops::Range};

#[derive(Debug)]
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

impl Default for Cursor {
    fn default() -> Self {
        Cursor::new(0)
    }
}
