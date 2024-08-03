mod eol;

use std::{ops::Range, sync::OnceLock};

use crate::{Bytes, PieceTreeSlice, ReadOnlyPieceTree};
use aho_corasick::{automaton::Automaton, nfa::contiguous::NFA, Anchored};

pub use self::eol::EndOfLine;

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

fn nfa_fwd() -> &'static NFA {
    static NFA: OnceLock<NFA> = OnceLock::new();
    NFA.get_or_init(|| NFA::new(EOLS).unwrap())
}

fn nfa_bwd() -> &'static NFA {
    static NFA: OnceLock<NFA> = OnceLock::new();
    NFA.get_or_init(|| {
        let eol_rev: Vec<Vec<u8>> = EOLS
            .into_iter()
            .map(|eol| {
                let bytes: &[u8] = eol.as_ref();
                bytes.iter().cloned().rev().collect()
            })
            .collect();
        NFA::new(eol_rev).unwrap()
    })
}

/// Advances bytes iterator to the next end of line and over it.
/// If an EOL is found returns the form of eol and the range it spans over.
pub fn next_eol(bytes: &mut Bytes) -> Option<EOLMatch> {
    let nfa = nfa_fwd();
    let mut state = nfa.start_state(ANC).unwrap();
    loop {
        let byte = bytes.next()?;
        state = nfa.next_state(ANC, state, byte);

        if nfa.is_match(state) {
            let pat = nfa.match_pattern(state, 0);
            let mut eol = EOLS[pat.as_usize()];

            let crlf = eol == EndOfLine::CR && bytes.get().map(|b| b == LF).unwrap_or(false);
            if crlf {
                // Advance over lf
                bytes.next();
                eol = EndOfLine::CRLF;
            }

            let end = bytes.pos();

            return Some(EOLMatch {
                eol,
                range: end - eol.len()..end,
            });
        }
    }
}

/// Advances bytes iterator to the previous end of line and over it.
/// If an EOL is found returns the form of eol and the range it spans over.
pub fn prev_eol(bytes: &mut Bytes) -> Option<EOLMatch> {
    let nfa = nfa_bwd();
    let mut state = nfa.start_state(ANC).unwrap();
    loop {
        let byte = bytes.prev()?;
        state = nfa.next_state(ANC, state, byte);

        if nfa.is_match(state) {
            let pat = nfa.match_pattern(state, 0);
            let mut eol = EOLS[pat.as_usize()];

            if eol == EndOfLine::LF {
                if let Some(b) = bytes.prev() {
                    if b == CR {
                        eol = EndOfLine::CRLF;
                    } else {
                        bytes.next();
                    }
                }
            }

            let start = bytes.pos();

            return Some(EOLMatch {
                eol,
                range: start..start + eol.len(),
            });
        }
    }
}

pub(crate) fn line_at(slice: &PieceTreeSlice, pos: usize) -> usize {
    let mut line = 0;
    let slice = slice.slice(..pos);
    let mut bytes = slice.bytes();

    while let Some(_) = next_eol(&mut bytes) {
        line += 1;
    }

    line
}

#[derive(Debug, Clone)]
pub struct Lines<'a> {
    bytes: Bytes<'a>,
    slice: PieceTreeSlice<'a>,
    at_end: bool,
}

impl<'a> Lines<'a> {
    #[inline]
    pub fn new(pt: &'a ReadOnlyPieceTree, at: usize) -> Lines {
        let slice = pt.slice(..);
        let bytes = Bytes::new(pt, at);
        let mut lines = Lines {
            at_end: at == pt.len(),
            slice,
            bytes,
        };
        lines.goto_bol();
        lines
    }

    #[inline]
    pub fn new_from_slice(slice: &PieceTreeSlice<'a>, at: usize) -> Lines<'a> {
        let slice = slice.clone();
        let bytes = Bytes::new_from_slice(&slice, at);
        let mut lines = Lines {
            at_end: bytes.pos() == slice.len(),
            slice,
            bytes,
        };
        lines.goto_bol();
        lines
    }

    #[inline]
    fn goto_bol(&mut self) {
        if self.bytes.pos() == self.slice.len() {
            return;
        }

        if let Some(m) = prev_eol(&mut self.bytes) {
            self.at_end = false;
            self.bytes.at(m.range.end);
        }
    }

    pub fn next(&mut self) -> Option<PieceTreeSlice> {
        let start = self.bytes.pos();

        match next_eol(&mut self.bytes) {
            Some(mat) => Some(self.slice.slice(start..mat.range.end)),
            None => {
                let end = self.bytes.pos();
                if start == end && self.at_end {
                    None
                } else {
                    self.at_end = end == self.slice.len();
                    Some(self.slice.slice(start..end))
                }
            }
        }
    }

    pub fn prev(&mut self) -> Option<PieceTreeSlice> {
        let end = self.bytes.pos();

        // Skip over previous eol
        if !self.at_end {
            prev_eol(&mut self.bytes);
        }
        self.at_end = false;

        match prev_eol(&mut self.bytes) {
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
pub struct EOLMatch {
    pub eol: EndOfLine,
    pub range: Range<usize>,
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
        assert_eq!(
            lines.next().as_ref().map(String::from),
            Some("".to_string())
        );

        assert!(lines.next().is_none());
    }

    #[test]
    fn lines_prev() {
        let mut pt = PieceTree::new();
        pt.insert(0, "foo\u{000A}bar\u{000B}baz\u{000C}this\u{000D}is\u{000D}\u{000A}another\u{0085}line\u{2028}boing\u{2029}\u{000A}");

        let mut lines = pt.lines_at(pt.len());

        assert_eq!(
            lines.prev().as_ref().map(String::from),
            Some("".to_string())
        );
        assert_eq!(
            lines.prev().as_ref().map(String::from),
            Some("\u{000A}".to_string())
        );
        assert_eq!(
            lines.prev().as_ref().map(String::from),
            Some("boing\u{2029}".to_string())
        );
        assert_eq!(
            lines.prev().as_ref().map(String::from),
            Some("line\u{2028}".to_string())
        );
        assert_eq!(
            lines.prev().as_ref().map(String::from),
            Some("another\u{0085}".to_string())
        );
        assert_eq!(
            lines.prev().as_ref().map(String::from),
            Some("is\u{000D}\u{000A}".to_string())
        );
        assert_eq!(
            lines.prev().as_ref().map(String::from),
            Some("this\u{000D}".to_string())
        );
        assert_eq!(
            lines.prev().as_ref().map(String::from),
            Some("baz\u{000C}".to_string())
        );
        assert_eq!(
            lines.prev().as_ref().map(String::from),
            Some("bar\u{000B}".to_string())
        );
        assert_eq!(
            lines.prev().as_ref().map(String::from),
            Some("foo\u{000A}".to_string())
        );

        assert!(lines.prev().is_none());
    }

    #[test]
    fn lines_middle() {
        let mut pt = PieceTree::new();
        pt.insert(
            0,
            b"foobarbaz\r\nHello world this is a long line with a lot of text\r\nthis",
        );
        let mut lines = pt.lines_at(25);

        assert_eq!(
            lines.next().as_ref().map(String::from),
            Some("Hello world this is a long line with a lot of text\r\n".to_string())
        );
        assert_eq!(
            lines.next().as_ref().map(String::from),
            Some("this".to_string())
        );

        assert!(lines.next().is_none());
    }
}
