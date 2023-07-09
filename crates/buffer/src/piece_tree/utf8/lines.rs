use std::ops::Range;

use crate::{Bytes, PieceTreeSlice, ReadOnlyPieceTree};
use aho_corasick::{
    automaton::{Automaton, StateID},
    nfa::contiguous::NFA,
    Anchored,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndOfLine {
    LF,   // LF: Line Feed, U+000A (UTF-8 in hex: 0A)
    VT,   // VT: Vertical Tab, U+000B (UTF-8 in hex: 0B)
    FF,   // FF: Form Feed, U+000C (UTF-8 in hex: 0C)
    CR,   // CR: Carriage Return, U+000D (UTF-8 in hex: 0D)
    CRLF, // CR+LF: CR (U+000D) followed by LF (U+000A) (UTF-8 in hex: 0D 0A)
    NEL,  // NEL: Next Line, U+0085 (UTF-8 in hex: C2 85)
    LS,   // LS: Line Separator, U+2028 (UTF-8 in hex: E2 80 A8)
    PS,   // PS: Paragraph Separator, U+2029 (UTF-8 in hex: E2 80 A9)
}

impl EndOfLine {
    pub fn is_eol<B: AsRef<[u8]>>(bytes: B) -> bool {
        let _bytes = bytes.as_ref();
        todo!()
    }

    pub fn is_slice_eol(_slice: &PieceTreeSlice) -> bool {
        todo!()
    }
}

impl AsRef<str> for EndOfLine {
    fn as_ref(&self) -> &str {
        use EndOfLine::*;

        match self {
            LF => "\u{000A}",
            VT => "\u{000B}",
            FF => "\u{000C}",
            CR => "\u{000D}",
            CRLF => "\u{000D}\u{000A}",
            NEL => "\u{0085}",
            LS => "\u{2028}",
            PS => "\u{2029}",
        }
    }
}

impl AsRef<[u8]> for EndOfLine {
    fn as_ref(&self) -> &[u8] {
        let string: &str = self.as_ref();
        string.as_bytes()
    }
}

#[derive(Debug, Clone)]
pub struct Lines<'a> {
    nfa_rev: NFA,
    nfa: NFA,
    state: StateID,
    state_rev: StateID,
    bytes: Bytes<'a>,
    pt: &'a ReadOnlyPieceTree,
}

impl<'a> Lines<'a> {
    const EOLS: [EndOfLine; 7] = [
        EndOfLine::LF,
        EndOfLine::VT,
        EndOfLine::FF,
        EndOfLine::CR,
        EndOfLine::NEL,
        EndOfLine::LS,
        EndOfLine::PS,
    ];

    fn build_rev() -> NFA {
        let eol_rev: Vec<Vec<u8>> = Self::EOLS
            .into_iter()
            .map(|eol| {
                let bytes: &[u8] = eol.as_ref();
                bytes.iter().cloned().rev().collect()
            })
            .collect();
        NFA::new(eol_rev).unwrap()
    }

    pub fn new(pt: &'a ReadOnlyPieceTree, at: usize) -> Lines {
        let nfa = NFA::new(Self::EOLS).unwrap();
        let nfa_rev = Self::build_rev();
        let state = nfa.start_state(Anchored::No).unwrap();
        let state_rev = nfa_rev.start_state(Anchored::No).unwrap();
        let bytes = Bytes::new(pt, at);
        Lines {
            pt,
            bytes,
            nfa,
            nfa_rev,
            state_rev,
            state,
        }
    }

    pub fn new_from_slice(pt: &'a ReadOnlyPieceTree, at: usize, range: Range<usize>) -> Lines {
        let nfa = NFA::new(Self::EOLS).unwrap();
        let nfa_rev = Self::build_rev();
        let state = nfa.start_state(Anchored::No).unwrap();
        let state_rev = nfa_rev.start_state(Anchored::No).unwrap();
        let bytes = Bytes::new_from_slice(pt, at, range);
        Lines {
            pt,
            bytes,
            nfa,
            nfa_rev,
            state,
            state_rev,
        }
    }

    pub fn next(&mut self) -> Option<PieceTreeSlice> {
        const LF: u8 = 0x0A;
        let start = self.bytes.pos();

        match self.nfa_next() {
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

                Some(self.pt.slice(start..end))
            }
            None => {
                let end = self.bytes.pos();
                if start == end {
                    // At end
                    None
                } else {
                    Some(self.pt.slice(start..end))
                }
            }
        }
    }

    fn nfa_next(&mut self) -> Option<EOLMatch> {
        loop {
            let byte = self.bytes.next()?;
            self.state = self.nfa.next_state(Anchored::No, self.state, byte);

            if self.nfa.is_match(self.state) {
                let pat = self.nfa.match_pattern(self.state, 0);
                let plen = self.nfa.pattern_len(pat);
                let pos = self.bytes.pos();
                return Some(EOLMatch {
                    eol: Self::EOLS[pat.as_usize()],
                    range: pos - plen..pos,
                });
            }
        }
    }

    pub fn prev(&mut self) -> Option<PieceTreeSlice> {
        const CR: u8 = 0x0D;
        let end = self.bytes.pos();

        match self.nfa_next() {
            Some(mat) => {
                let crlf =
                    mat.eol == EndOfLine::LF && self.bytes.get().map(|b| b == CR).unwrap_or(false);

                let start = if crlf {
                    // Advance over lf
                    self.bytes.next();
                    mat.range.start - 1
                } else {
                    mat.range.start
                };

                Some(self.pt.slice(start..end))
            }
            None => {
                let start = self.bytes.pos();
                if start == end {
                    // At start
                    None
                } else {
                    Some(self.pt.slice(start..end))
                }
            }
        }
    }

    fn nfa_prev(&mut self) -> Option<EOLMatch> {
        loop {
            let byte = self.bytes.prev()?;
            self.state_rev = self.nfa_rev.next_state(Anchored::No, self.state_rev, byte);

            if self.nfa_rev.is_match(self.state_rev) {
                let pat = self.nfa_rev.match_pattern(self.state_rev, 0);
                let plen = self.nfa_rev.pattern_len(pat);
                let pos = self.bytes.pos();
                return Some(EOLMatch {
                    eol: Self::EOLS[pat.as_usize()],
                    range: pos..pos + plen,
                });
            }
        }
    }
}

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
        pt.insert(0, b"Hello\nworld\r\nthis");

        let mut lines = pt.lines();
        while let Some(line) = lines.next() {
            println!("LINE: {:?}", String::from(&line))
        }
    }

    #[test]
    fn lines_prev() {
        let mut pt = PieceTree::new();
        pt.insert(0, b"Hello\nworld\r\nthis");

        let mut lines = pt.lines_at(pt.len());
        while let Some(line) = lines.prev() {
            println!("LINE: {:?}", String::from(&line))
        }
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
