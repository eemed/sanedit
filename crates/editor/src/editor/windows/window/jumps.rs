use std::{collections::VecDeque, sync::Arc};

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
    bid: BufferId,
    jumps: Vec<Jump>,
}

impl JumpGroup {
    pub fn new(id: BufferId, jumps: Vec<Jump>) -> JumpGroup {
        JumpGroup { bid: id, jumps }
    }

    pub fn buffer_id(&self) -> BufferId {
        self.bid
    }

    pub fn jumps(&self) -> &[Jump] {
        &self.jumps
    }

    pub fn to_cursors(&self, buf: &Buffer) -> Cursors {
        let mut cursors = Cursors::default();

        for (i, jump) in self.jumps().iter().enumerate() {
            let start = buf.mark_to_pos(jump.start()).pos();
            let end = jump.end().map(|mark| buf.mark_to_pos(mark));

            let cursor = if let Some(end) = end {
                Cursor::new_select(&Range::new(start, end.pos()))
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

pub(crate) struct Iter<'a> {
    jumps: &'a VecDeque<Arc<JumpGroup>>,
    position: Option<usize>,
}

impl<'a> Iter<'a> {
    pub fn next(&mut self) -> Option<(JumpCursor, &JumpGroup)> {
        let index = self.position? + 1;
        if index >= self.jumps.len() {
            return None;
        }

        let group = self.jumps.get(index)?;
        let cursor = JumpCursor(Arc::as_ptr(group));
        Some((cursor, group))
    }

    pub fn prev(&mut self) -> Option<(JumpCursor, &JumpGroup)> {
        let index = match self.position {
            Some(pos) => {
                if pos == 0 {
                    return None;
                }
                pos - 1
            }
            None => self.jumps.len() - 1,
        };

        let group = self.jumps.get(index)?;
        let cursor = JumpCursor(Arc::as_ptr(group));
        Some((cursor, group))
    }
}

/// Object that remembers a position in jumps
pub(crate) struct JumpCursor(*const JumpGroup);

#[derive(Debug)]
pub(crate) struct Jumps {
    jumps: VecDeque<Arc<JumpGroup>>,

    // Dont store too many
    cap: usize,

    /// None = back
    position: Option<usize>,
}

impl Jumps {
    pub fn new(groups: Vec<JumpGroup>) -> Jumps {
        let groups: Vec<Arc<JumpGroup>> = groups.into_iter().map(Arc::new).collect();
        let len = groups.len();
        let mut deque = VecDeque::with_capacity(len);
        deque.extend(groups);

        Jumps {
            jumps: deque,
            cap: len,
            position: None,
        }
    }

    pub fn with_capacity(cap: usize) -> Jumps {
        Jumps {
            jumps: VecDeque::new(),
            cap,
            position: None,
        }
    }

    /// Takes the front jump group out of jumps
    pub fn take(&mut self) -> Option<JumpGroup> {
        let group = self.jumps.pop_back()?;
        Arc::into_inner(group)
    }

    pub fn is_empty(&self) -> bool {
        self.jumps.is_empty()
    }

    pub fn push(&mut self, group: JumpGroup) {
        while self.jumps.len() >= self.cap {
            self.jumps.pop_front();
        }

        self.jumps.push_back(Arc::new(group));
    }

    pub fn goto_start(&mut self) {
        self.position = None;
    }

    /// New iterator starting at current position
    pub fn iter(&self) -> Iter {
        Iter {
            jumps: &self.jumps,
            position: self.position.clone(),
        }
    }

    pub fn goto(&mut self, cursor: JumpCursor) -> Option<&JumpGroup> {
        let i = self.jumps.iter().rev().position(|group| {
            let ptr = Arc::as_ptr(group);
            std::ptr::eq(ptr, cursor.0)
        })?;
        let index = self.jumps.len() - 1 - i;
        self.position = Some(index);
        let group = self.jumps.get(index)?.as_ref();
        Some(group)
    }

    pub fn current(&self) -> Option<&JumpGroup> {
        let current = self.jumps.get(self.position?)?;
        Some(current.as_ref())
    }
}
