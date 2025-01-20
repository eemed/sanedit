use std::collections::VecDeque;

use sanedit_buffer::Mark;
use sanedit_core::BufferRange;

#[derive(Debug)]
pub(crate) enum Jump {
    MarkRange(Mark, Mark),
    Range(BufferRange),
    Position(u64),
    Mark(Mark),
}

#[derive(Debug)]
pub(crate) struct Jumps {
    jumps: VecDeque<Jump>,
}
