use std::ops::Range;

use crate::ByteSource;

use super::{Jit, ParsingMachine, SubjectPosition};

pub type CaptureID = usize;
pub type CaptureList = Vec<Capture>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Capture {
    pub(crate) id: CaptureID,
    pub(crate) start: SubjectPosition,
    pub(crate) end: SubjectPosition,
}

impl Capture {
    pub fn id(&self) -> CaptureID {
        self.id
    }

    pub fn range(&self) -> Range<u64> {
        self.start..self.end
    }
}

impl Ord for Capture {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.start
            .cmp(&other.start)
            .then(other.end.cmp(&self.end))
            .then(self.id.cmp(&other.id))
    }
}

impl PartialOrd for Capture {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug)]
pub(crate) enum ParserRef<'a> {
    Interpreted(&'a ParsingMachine),
    Jit(&'a Jit),
}

/// Iterate over matched captures.
/// Yields captures only when matching succeeds otherwise tries again at the next position
#[derive(Debug)]
pub struct CaptureIter<'a, B: ByteSource> {
    pub(crate) parser: ParserRef<'a>,
    pub(crate) source: B,
    pub(crate) sp: u64,
}

impl<'a, B: ByteSource> Iterator for CaptureIter<'a, B> {
    type Item = CaptureList;

    fn next(&mut self) -> Option<Self::Item> {
        match self.parser {
            ParserRef::Interpreted(parsing_machine) => {
                match parsing_machine.do_parse(&mut self.source, self.sp) {
                    Ok((caps, sp)) => {
                        self.sp = sp;
                        if caps.is_empty() {
                            None
                        } else {
                            Some(caps)
                        }
                    }
                    Err(_) => {
                        self.sp = self.source.len();
                        None
                    }
                }
            }
            ParserRef::Jit(jit) => match jit.do_parse(&mut self.source, self.sp, true) {
                Ok((caps, sp)) => {
                    self.sp = sp;
                    if caps.is_empty() {
                        None
                    } else {
                        Some(caps)
                    }
                }
                Err(_) => {
                    self.sp = self.source.len();
                    None
                }
            },
        }
    }
}
