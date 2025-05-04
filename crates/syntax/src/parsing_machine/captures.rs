use std::ops::Range;

use crate::{ByteReader, SubjectPosition};

use super::Parser;

pub type CaptureID = usize;
pub type CaptureList = Vec<Capture>;

#[derive(Debug, Clone)]
pub struct Capture {
    pub(crate) id: CaptureID,
    pub(crate) start: SubjectPosition,
    pub(crate) len: Option<u64>,
    pub(crate) reopen: bool,
}

impl Capture {
    pub(crate) fn new(id: CaptureID, start: SubjectPosition) -> Capture {
        Capture {
            id,
            start,
            len: None,
            reopen: false,
        }
    }

    pub(crate) fn new_reopenable(id: CaptureID, start: SubjectPosition) -> Capture {
        Capture {
            id,
            start,
            len: None,
            reopen: true,
        }
    }

    pub(crate) fn is_reopenable(&self) -> bool {
        self.reopen
    }

    pub(crate) fn reopen(&mut self) {
        self.len = None;
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

/// Iterate over matched captures.
/// Yields captures only when matching succeeds otherwise tries again at the next position
#[derive(Debug)]
pub struct CaptureIter<'a, B: ByteReader> {
    pub(super) parser: &'a Parser,
    pub(super) reader: B,
    pub(super) sp: u64,
}

impl<'a, B: ByteReader> Iterator for CaptureIter<'a, B> {
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
