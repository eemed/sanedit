use std::collections::VecDeque;

use sanedit_buffer::Mark;
use sanedit_core::{Cursor, Range};

use crate::editor::buffers::{Buffer, BufferId};

use super::Cursors;

/// jump to a position or selection in buffer
#[derive(Debug, Clone)]
pub(crate) struct Jump {
    start: Mark,
    /// If jump selects a portion of the text end is set
    end: Option<Mark>,
}

impl Jump {
    pub fn new(start: Mark, end: Option<Mark>) -> Jump {
        Jump { start, end }
    }

    pub fn start(&self) -> &Mark {
        &self.start
    }

    pub fn end(&self) -> Option<&Mark> {
        self.end.as_ref()
    }
}

/// A group of jumps meant to be used at the same time.
/// Mostly to place a cursor on each jump simultaneously
#[derive(Debug, Clone)]
pub(crate) struct JumpGroup {
    #[allow(dead_code)]
    bid: BufferId,
    jumps: Vec<Jump>,
}

impl JumpGroup {
    pub fn new(id: BufferId, jumps: Vec<Jump>) -> JumpGroup {
        JumpGroup { bid: id, jumps }
    }

    pub fn jumps(&self) -> &[Jump] {
        &self.jumps
    }

    pub fn to_cursors(&self, buf: &Buffer) -> Cursors {
        let mut cursors = Cursors::default();

        for (i, jump) in self.jumps().iter().enumerate() {
            let start = buf.mark_to_pos(jump.start());
            let end = jump.end().map(|mark| buf.mark_to_pos(mark));

            let cursor = if let Some(end) = end {
                Cursor::new_select(&Range::new(start, end))
            } else {
                Cursor::new(start)
            };

            let first = i == 0;
            if first {
                cursors.replace_primary(cursor);
            } else {
                cursors.push(cursor);
            }
        }

        cursors
    }
}

#[derive(Debug, Default)]
pub(crate) struct Jumps {
    jumps: VecDeque<JumpGroup>,
    // Dont store too many
    cap: usize,

    /// None = front before 0
    position: Option<usize>,
}

impl Jumps {
    pub fn new(groups: Vec<JumpGroup>) -> Jumps {
        let len = groups.len();
        let mut deque = VecDeque::with_capacity(len);
        deque.extend(groups);

        Jumps {
            jumps: deque,
            cap: len,
            position: None,
        }
    }

    /// Takes the front jump group out of jumps
    pub fn take_front(&mut self) -> Option<JumpGroup> {
        self.jumps.pop_front()
    }

    pub fn is_empty(&self) -> bool {
        self.jumps.is_empty()
    }

    pub fn push(&mut self, group: JumpGroup) {
        while self.jumps.len() >= self.cap {
            self.jumps.pop_back();
        }

        self.jumps.push_front(group);
        self.position = None;
    }

    /// Goto the previous jump group inserted
    pub fn prev(&mut self) -> Option<&JumpGroup> {
        self.position = match self.position {
            Some(pos) => Some(pos + 1),
            None => Some(0),
        };

        self.jumps.get(self.position?)
    }

    /// Goto the next jump group, return Some only if prev was called before
    pub fn next(&mut self) -> Option<&JumpGroup> {
        let pos = self.position?;
        if pos == 0 {
            self.position = None;
        } else {
            self.position = Some(pos - 1);
        }

        self.jumps.get(self.position?)
    }
}
