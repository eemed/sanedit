use std::ops::Range;

use crate::SubjectPosition;

pub type CaptureID = usize;
pub type CaptureList = Vec<Capture>;

#[derive(Debug, Clone)]
pub struct Capture {
    pub(crate) id: CaptureID,
    pub(crate) start: SubjectPosition,
    pub(crate) len: Option<u64>,
}

impl Capture {
    pub(crate) fn new(id: CaptureID, start: SubjectPosition) -> Capture {
        Capture {
            id,
            start,
            len: None,
        }
    }
    pub(crate) fn is_closed(&self) -> bool {
        self.len.is_some()
    }

    pub(crate) fn close(&mut self, end: SubjectPosition) {
        self.len = Some(end - self.start);
    }

    pub fn id(&self) -> CaptureID {
        self.id
    }

    pub fn range(&self) -> Range<u64> {
        let len = self
            .len
            .expect("Should not return a capture without length");

        self.start..self.start + len
    }
}
