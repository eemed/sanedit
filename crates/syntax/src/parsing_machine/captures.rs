use std::ops::Range;

use crate::source::Source;

use super::{Jit, ParsingMachine, SubjectPosition};

pub type CaptureID = usize;
pub type CaptureList = Vec<Capture>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Captures {
    pub captures: CaptureList,
    pub injections: Vec<(String, Captures)>,
}

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
pub struct CaptureIter<'a, 'b, S: Source> {
    pub(crate) parser: ParserRef<'a>,
    pub(crate) source: &'b mut S,
    pub(crate) sp: u64,
    pub(crate) sp_rev: u64,
}

impl<'a, 'b, S: Source> Iterator for CaptureIter<'a, 'b, S> {
    type Item = CaptureList;

    fn next(&mut self) -> Option<Self::Item> {
        match self.parser {
            ParserRef::Interpreted(parsing_machine) => {
                match parsing_machine.do_parse(self.source, self.sp) {
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
            ParserRef::Jit(jit) => match jit.do_parse(self.source, self.sp, true) {
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

impl<'a, 'b, S: Source> DoubleEndedIterator for CaptureIter<'a, 'b, S> {
    fn next_back(&mut self) -> Option<Self::Item> {
        while self.sp_rev != 0 {
            let mut found = None;
            let mut pos = 0;
            loop {
                match self.parser {
                    ParserRef::Interpreted(parsing_machine) => {
                        match parsing_machine.do_parse(self.source, pos) {
                            Ok((caps, sp)) => {
                                pos = sp;
                                if caps.is_empty() {
                                    continue;
                                }

                                found = Some((caps, sp));
                            }
                            Err(_) => break,
                        }
                    }
                    ParserRef::Jit(jit) => match jit.do_parse(self.source, pos, true) {
                        Ok((caps, sp)) => {
                            pos = sp;
                            if caps.is_empty() {
                                continue;
                            }

                            found = Some((caps, sp));
                        }
                        Err(_) => break,
                    },
                }
            }

            match found {
                Some((caps, sp)) => {
                    self.sp_rev = sp;
                    return Some(caps);
                }
                None => {
                    self.sp_rev = 0;
                }
            }
        }

        None
    }
}
