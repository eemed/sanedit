use std::collections::VecDeque;

use sanedit_buffer::Mark;

use crate::editor::buffers::BufferId;

/// jump to a position or selection in buffer
#[derive(Debug)]
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
#[derive(Debug)]
pub(crate) struct JumpGroup {
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
}

#[derive(Debug)]
pub(crate) struct Jumps {
    jumps: VecDeque<JumpGroup>,
}

impl Jumps {
    pub fn new(groups: Vec<JumpGroup>) -> Jumps {
        let mut deque = VecDeque::with_capacity(groups.len());
        deque.extend(groups);

        Jumps { jumps: deque }
    }

    pub fn next(&mut self) -> Option<JumpGroup> {
        self.jumps.pop_front()
    }
}
