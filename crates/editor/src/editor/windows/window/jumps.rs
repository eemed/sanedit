use sanedit_buffer::Mark;
use sanedit_core::{Cursor, Range};
use sanedit_utils::ring::{Ref, RingBuffer};

use crate::editor::buffers::{Buffer, BufferId};

use super::Cursors;

/// jump to a position or selection in buffer
#[derive(Debug, Clone, PartialEq)]
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
#[derive(Debug, Clone, PartialEq)]
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

#[derive(Debug)]
pub(crate) struct Jumps {
    jumps: RingBuffer<JumpGroup>,

    /// None = back
    position: Option<Ref>,
}

impl Jumps {
    pub fn new(groups: Vec<JumpGroup>) -> Jumps {
        let len = groups.len();
        let mut deque = RingBuffer::with_capacity(len);
        deque.extend(groups);

        Jumps {
            jumps: deque,
            position: None,
        }
    }

    pub fn with_capacity(cap: usize) -> Jumps {
        Jumps {
            jumps: RingBuffer::with_capacity(cap),
            position: None,
        }
    }

    /// Takes the front jump group out of jumps
    pub fn take(&mut self) -> Option<JumpGroup> {
        self.jumps.take()
    }

    pub fn is_empty(&self) -> bool {
        !self.jumps.is_empty()
    }

    pub fn push(&mut self, group: JumpGroup) {
    log::info!("push: {group:?}");
        self.jumps.push_overwrite(group);
    }

    pub fn goto_start(&mut self) {
        self.position = None;
    }

    pub fn goto(&mut self, reference: Ref) -> Option<&JumpGroup> {
        let group = self.jumps.read_reference(&reference)?;
        self.position = Some(reference);
        Some(group)
    }

    pub fn current(&self) -> Option<(Ref, &JumpGroup)> {
        let reference = self.position.clone()?;
        let group = self.jumps.read_reference(&reference)?;
        Some((reference, group))
    }

    pub fn last(&self) -> Option<(Ref, &JumpGroup)> {
        self.jumps.last()
    }

    pub fn next(&self, reference: &Ref) -> Option<(Ref, &JumpGroup)> {
        self.jumps.next(reference)
    }

    pub fn prev(&self, reference: &Ref) -> Option<(Ref, &JumpGroup)> {
        self.jumps.previous(reference)
    }
}
