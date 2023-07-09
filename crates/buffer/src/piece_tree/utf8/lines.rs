mod eol;

use std::ops::Range;

use crate::{Bytes, PieceTreeSlice, ReadOnlyPieceTree};
use aho_corasick::{automaton::Automaton, nfa::contiguous::NFA, Anchored};

use self::eol::EndOfLine;

const LF: u8 = 0x0A;
const CR: u8 = 0x0D;
const ANC: Anchored = Anchored::No;
const EOLS: [EndOfLine; 7] = [
    EndOfLine::LF,
    EndOfLine::VT,
    EndOfLine::FF,
    EndOfLine::CR,
    EndOfLine::NEL,
    EndOfLine::LS,
    EndOfLine::PS,
    // Missing CRLF on purpose, handled separately
];

lazy_static! {
    static ref NFA_FWD: NFA = NFA::new(EOLS).unwrap();
    static ref NFA_BWD: NFA = {
        let eol_rev: Vec<Vec<u8>> = EOLS
            .into_iter()
            .map(|eol| {
                let bytes: &[u8] = eol.as_ref();
                bytes.iter().cloned().rev().collect()
            })
            .collect();
        NFA::new(eol_rev).unwrap()
    };
}

#[inline]
pub fn start_of_line(slice: &PieceTreeSlice, pos: usize) -> usize {
    let mut bytes = slice.bytes_at(pos);
    match nfa_prev_eol(&mut bytes) {
        Some(m) => m.range.start,
        None => 0,
    }
}

#[inline]
pub fn end_of_line(slice: &PieceTreeSlice, pos: usize) -> usize {
    let mut bytes = slice.bytes_at(pos);
    match nfa_next_eol(&mut bytes) {
        Some(m) => {
            let crlf = m.eol == EndOfLine::CR && bytes.get().map(|b| b == LF).unwrap_or(false);

            if crlf {
                m.range.end + 1
            } else {
                m.range.end
            }
        }
        None => 0,
    }
}

fn nfa_next_eol(bytes: &mut Bytes) -> Option<EOLMatch> {
    let mut state = NFA_FWD.start_state(ANC).unwrap();
    loop {
        let byte = bytes.next()?;
        state = NFA_FWD.next_state(ANC, state, byte);

        if NFA_FWD.is_match(state) {
            let pat = NFA_FWD.match_pattern(state, 0);
            let plen = NFA_FWD.pattern_len(pat);
            let pos = bytes.pos();
            return Some(EOLMatch {
                eol: EOLS[pat.as_usize()],
                range: pos - plen..pos,
            });
        }
    }
}

fn nfa_prev_eol(bytes: &mut Bytes) -> Option<EOLMatch> {
    let mut state = NFA_BWD.start_state(ANC).unwrap();
    loop {
        let byte = bytes.prev()?;
        state = NFA_BWD.next_state(ANC, state, byte);

        if NFA_BWD.is_match(state) {
            let pat = NFA_BWD.match_pattern(state, 0);
            let plen = NFA_BWD.pattern_len(pat);
            let pos = bytes.pos();
            return Some(EOLMatch {
                eol: EOLS[pat.as_usize()],
                range: pos..pos + plen,
            });
        }
    }
}

#[derive(Debug, Clone)]
pub struct Lines<'a> {
    bytes: Bytes<'a>,
    slice: PieceTreeSlice<'a>,
}

impl<'a> Lines<'a> {
    #[inline]
    pub fn new(pt: &'a ReadOnlyPieceTree, at: usize) -> Lines {
        let slice = pt.slice(..);
        let bytes = Bytes::new(pt, at);
        let mut lines = Lines { slice, bytes };
        lines.goto_bol();
        lines
    }

    #[inline]
    pub fn new_from_slice(slice: &PieceTreeSlice<'a>, at: usize) -> Lines<'a> {
        let slice = slice.clone();
        let bytes = Bytes::new_from_slice(&slice, at);
        let mut lines = Lines { slice, bytes };
        lines.goto_bol();
        lines
    }

    #[inline]
    fn goto_bol(&mut self) {
        if self.bytes.pos() == self.slice.len() {
            return;
        }

        if let Some(m) = nfa_prev_eol(&mut self.bytes) {
            self.bytes.at(m.range.end);
        }
    }

    pub fn next(&mut self) -> Option<PieceTreeSlice> {
        let start = self.bytes.pos();

        match nfa_next_eol(&mut self.bytes) {
            Some(mat) => {
                let crlf =
                    mat.eol == EndOfLine::CR && self.bytes.get().map(|b| b == LF).unwrap_or(false);

                let end = if crlf {
                    // Advance over lf
                    self.bytes.next();
                    mat.range.end + 1
                } else {
                    mat.range.end
                };

                Some(self.slice.slice(start..end))
            }
            None => {
                let end = self.bytes.pos();
                if start == end {
                    // At end
                    None
                } else {
                    Some(self.slice.slice(start..end))
                }
            }
        }
    }

    pub fn prev(&mut self) -> Option<PieceTreeSlice> {
        let end = self.bytes.pos();

        // Skip over previous eol
        // if self.bytes.pos() != self.slice.len() {
        if let Some(m) = nfa_prev_eol(&mut self.bytes) {
            // Handle crlf
            if m.eol == EndOfLine::LF {
                if let Some(b) = self.bytes.prev() {
                    if b != CR {
                        self.bytes.prev();
                    }
                }
            }
        }
        // }

        match nfa_prev_eol(&mut self.bytes) {
            Some(mat) => {
                let start = mat.range.end;

                // Move bytes to start of line
                for _ in 0..mat.range.len() {
                    self.bytes.next();
                }

                Some(self.slice.slice(start..end))
            }
            None => {
                let start = self.bytes.pos();
                if start == end {
                    // At start
                    None
                } else {
                    Some(self.slice.slice(start..end))
                }
            }
        }
    }
}

#[derive(Debug)]
struct EOLMatch {
    eol: EndOfLine,
    range: Range<usize>,
}

#[cfg(test)]
mod test {
    use crate::PieceTree;

    #[test]
    fn lines_next() {
        let mut pt = PieceTree::new();
        pt.insert(0, "foo\u{000A}bar\u{000B}baz\u{000C}this\u{000D}is\u{000D}\u{000A}another\u{0085}line\u{2028}boing\u{2029}\u{000A}");

        let mut lines = pt.lines();

        assert_eq!(
            lines.next().as_ref().map(String::from),
            Some("foo\u{000A}".to_string())
        );

        assert_eq!(
            lines.next().as_ref().map(String::from),
            Some("bar\u{000B}".to_string())
        );
        assert_eq!(
            lines.next().as_ref().map(String::from),
            Some("baz\u{000C}".to_string())
        );
        assert_eq!(
            lines.next().as_ref().map(String::from),
            Some("this\u{000D}".to_string())
        );
        assert_eq!(
            lines.next().as_ref().map(String::from),
            Some("is\u{000D}\u{000A}".to_string())
        );
        assert_eq!(
            lines.next().as_ref().map(String::from),
            Some("another\u{0085}".to_string())
        );
        assert_eq!(
            lines.next().as_ref().map(String::from),
            Some("line\u{2028}".to_string())
        );
        assert_eq!(
            lines.next().as_ref().map(String::from),
            Some("boing\u{2029}".to_string())
        );
        assert_eq!(
            lines.next().as_ref().map(String::from),
            Some("\u{000A}".to_string())
        );

        assert!(lines.next().is_none());
    }

    #[test]
    fn lines_prev() {
        let mut pt = PieceTree::new();
        pt.insert(0, "foo\u{000A}bar\u{000B}baz\u{000C}this\u{000D}is\u{000D}\u{000A}another\u{0085}line\u{2028}boing\u{2029}\u{000A}");

        let mut lines = pt.lines_at(pt.len());

        while let Some(l) = lines.prev() {
            println!("line: {:?}", String::from(&l));
        }

        // assert_eq!(
        //     lines.prev().as_ref().map(String::from),
        //     Some("\u{000A}".to_string())
        // );
        // assert_eq!(
        //     lines.prev().as_ref().map(String::from),
        //     Some("boing\u{2029}".to_string())
        // );
        // assert_eq!(
        //     lines.prev().as_ref().map(String::from),
        //     Some("line\u{2028}".to_string())
        // );
        // assert_eq!(
        //     lines.prev().as_ref().map(String::from),
        //     Some("another\u{0085}".to_string())
        // );
        // assert_eq!(
        //     lines.prev().as_ref().map(String::from),
        //     Some("is\u{000D}\u{000A}".to_string())
        // );
        // assert_eq!(
        //     lines.prev().as_ref().map(String::from),
        //     Some("this\u{000D}".to_string())
        // );
        // assert_eq!(
        //     lines.prev().as_ref().map(String::from),
        //     Some("baz\u{000C}".to_string())
        // );
        // assert_eq!(
        //     lines.prev().as_ref().map(String::from),
        //     Some("bar\u{000B}".to_string())
        // );
        // assert_eq!(
        //     lines.prev().as_ref().map(String::from),
        //     Some("foo\u{000A}".to_string())
        // );

        // assert!(lines.prev().is_none());
    }

    #[test]
    fn lines_middle() {
        let mut pt = PieceTree::new();
        pt.insert(
            0,
            b"foobarbaz\r\nHello world this is a long line with a lot of text\r\nthis",
        );

        let mut lines = pt.lines_at(25);
        while let Some(line) = lines.next() {
            println!("LINE: {:?}", String::from(&line))
        }
    }
}
