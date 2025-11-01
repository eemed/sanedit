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
pub struct CaptureIter<'a, S: Source> {
    pub(crate) parser: ParserRef<'a>,
    pub(crate) source: S,
    pub(crate) sp: u64,
    pub(crate) sp_rev: u64,
}

impl<'a, S: Source> Iterator for CaptureIter<'a, S> {
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

impl<'a, S: Source> DoubleEndedIterator for CaptureIter<'a, S> {
    fn next_back(&mut self) -> Option<Self::Item> {
        // const SIZE: u64 = 4096;
        // const OVERLAP: u64 = 2048;
        // let mut buf = vec![0u8; SIZE as usize];

        while self.sp_rev != 0 {
            // let start = self.sp_rev.saturating_sub(SIZE);
            // let mut chunk = if let Some(chunk) = self.source.as_single_chunk() {
            //     let start = start as usize;
            //     let end: usize = min(chunk.len(), start + SIZE as usize);
            //     std::borrow::Cow::from(&chunk[start..end])
            // } else {
            //     let n = self.source.copy_to(start, &mut buf);
            //     std::borrow::Cow::from(&buf[..n])
            // };

            let mut found = None;
            let mut pos = 0;
            loop {
                match self.parser {
                    ParserRef::Interpreted(parsing_machine) => {
                        match parsing_machine.do_parse(&mut self.source, pos) {
                            Ok((mut caps, sp)) => {
                                pos = sp;
                                if caps.is_empty() {
                                    continue;
                                }
                                // caps.iter_mut().for_each(|cap| {
                                //     cap.start += start;
                                //     cap.end += start;
                                // });
                                found = Some((caps, sp));
                            }
                            Err(_) => break,
                        }
                    }
                    ParserRef::Jit(jit) => match jit.do_parse(&mut self.source, pos, true) {
                        Ok((mut caps, sp)) => {
                            pos = sp;
                            if caps.is_empty() {
                                continue;
                            }
                            // caps.iter_mut().for_each(|cap| {
                            //     cap.start += start;
                            //     cap.end += start;
                            // });
                            found = Some((caps, sp));
                        }
                        Err(_) => break,
                    },
                }
            }

            match found {
                Some((caps, sp)) => {
                    // self.sp_rev = start + sp;
                    self.sp_rev = sp;
                    return Some(caps);
                }
                None => {
                    self.sp_rev = 0; 
                    // self.sp_rev.saturating_sub(SIZE - OVERLAP);
                }
            }
        }

        None
    }
}
