use std::{cmp, ops::Range};

use crate::editor::buffers::Buffer;

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
