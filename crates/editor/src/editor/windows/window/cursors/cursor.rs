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

    unanchor_on_move: bool,
}

impl Cursor {
    pub fn new(pos: usize) -> Cursor {
        Cursor {
            pos,
            col: None,
            anchor: None,
            unanchor_on_move: false,
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

        if self.unanchor_on_move {
            self.unanchor();
        }
    }

    pub fn goto_with_col(&mut self, pos: usize, col: usize) {
        self.pos = pos;
        self.col = Some(col);

        if self.unanchor_on_move {
            self.unanchor();
        }
    }

    pub fn anchor(&mut self) {
        self.anchor = Some(self.pos);
    }

    pub fn unanchor(&mut self) {
        self.unanchor_on_move = false;
        self.anchor = None;
    }

    pub fn set_unanchor_on_move(&mut self) {
        self.unanchor_on_move = true;
    }

    pub fn selection(&self) -> Option<Range<usize>> {
        let anchor = self.anchor?;
        let min = cmp::min(self.pos, anchor);
        let max = cmp::max(self.pos, anchor);
        Some(min..max)
    }

    /// Remove the selected text from the buffer and restore cursor to non
    /// selecting.
    pub fn remove_selection(&mut self, buf: &mut Buffer) {
        if let Some(sel) = self.selection() {
            let Range { start, .. } = sel;
            buf.remove(sel);
            self.unanchor();
            self.goto(start);
        }
    }
}

impl Default for Cursor {
    fn default() -> Self {
        Cursor::new(0)
    }
}
