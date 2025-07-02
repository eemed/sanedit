use std::ops::Range;

use crate::{ByteSource, SubjectPosition};

use super::Parser;

pub type CaptureID = usize;
pub type CaptureList = Vec<Capture>;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Capture {
    pub(crate) start: SubjectPosition,
    pub(crate) end: SubjectPosition,
    pub(crate) id: CaptureID,
}

impl Capture {
    pub fn id(&self) -> CaptureID {
        self.id
    }

    pub fn range(&self) -> Range<u64> {
        self.start..self.end
    }
}

/// Iterate over matched captures.
/// Yields captures only when matching succeeds otherwise tries again at the next position
#[derive(Debug)]
pub struct CaptureIter<'a, B: ByteSource> {
    pub(super) parser: &'a Parser,
    pub(super) reader: B,
    pub(super) sp: u64,
}

impl<'a, B: ByteSource> Iterator for CaptureIter<'a, B> {
    type Item = CaptureList;

    fn next(&mut self) -> Option<Self::Item> {
        match self.parser.do_parse(&mut self.reader, self.sp) {
            Ok((caps, sp)) => {
                self.sp = sp;
                Some(caps)
            }
            Err(_) => {
                self.sp = self.reader.len();
                None
            }
        }
    }
}
