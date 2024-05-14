use crate::SubjectPosition;

pub type CaptureID = usize;
pub type CaptureList = Vec<Capture>;

#[derive(Debug, Clone)]
pub struct Capture {
    pub(crate) id: CaptureID,
    pub(crate) start: SubjectPosition,
    pub(crate) len: Option<usize>,
}

impl Capture {
    pub fn new(id: CaptureID, start: SubjectPosition) -> Capture {
        Capture {
            id,
            start,
            len: None,
        }
    }

    pub fn is_closed(&self) -> bool {
        self.len.is_some()
    }

    pub fn close(&mut self, end: SubjectPosition) {
        self.len = Some(end - self.start);
    }
}
