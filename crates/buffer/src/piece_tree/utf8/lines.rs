mod eol;

use std::ops::Range;

use crate::{Bytes, PieceTreeSlice, PieceTreeView};

pub use self::eol::EndOfLine;

/// Advances bytes iterator to the next end of line and over it.
/// If an EOL is found returns the form of eol and the range it spans over.
pub fn next_eol(bytes: &mut Bytes) -> Option<EOLMatch> {
    loop {
        let byte = bytes.next()?;
        match byte {
            // LF VT FF
            0x0a => {
                let pos = bytes.pos();
                return Some(EOLMatch {
                    eol: EndOfLine::LF,
                    range: pos - 1..pos,
                });
            }
            0x0b => {
                let pos = bytes.pos();
                return Some(EOLMatch {
                    eol: EndOfLine::VT,
                    range: pos - 1..pos,
                });
            }
            0x0c => {
                let pos = bytes.pos();
                return Some(EOLMatch {
                    eol: EndOfLine::FF,
                    range: pos - 1..pos,
                });
            }
            // CR
            0x0d => {
                let crlf = bytes.get().map(|b| b == 0x0a).unwrap_or(false);
                if crlf {
                    bytes.next();
                    let pos = bytes.pos();
                    return Some(EOLMatch {
                        eol: EndOfLine::CRLF,
                        range: pos - 2..pos,
                    });
                } else {
                    let pos = bytes.pos();
                    return Some(EOLMatch {
                        eol: EndOfLine::CR,
                        range: pos - 1..pos,
                    });
                }
            }
            // NEL
            0xc2 => {
                let nel = bytes.get().map(|b| b == 0x85).unwrap_or(false);
                if nel {
                    bytes.next();
                    let pos = bytes.pos();
                    return Some(EOLMatch {
                        eol: EndOfLine::NEL,
                        range: pos - 2..pos,
                    });
                }
            }
            // LS PS
            0xe2 => {
                let cont = bytes.get().map(|b| b == 0x80).unwrap_or(false);

                if !cont {
                    continue;
                }

                bytes.next();

                match bytes.get() {
                    Some(0xa8) => {
                        bytes.next();
                        let pos = bytes.pos();
                        return Some(EOLMatch {
                            eol: EndOfLine::LS,
                            range: pos - 3..pos,
                        });
                    }
                    Some(0xa9) => {
                        bytes.next();
                        let pos = bytes.pos();
                        return Some(EOLMatch {
                            eol: EndOfLine::PS,
                            range: pos - 3..pos,
                        });
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
}

/// Advances bytes iterator to the previous end of line and over it.
/// If an EOL is found returns the form of eol and the range it spans over.
pub fn prev_eol(bytes: &mut Bytes) -> Option<EOLMatch> {
log::info!("PREV EOL: {}", bytes.pos());
    let mut byte = bytes.prev()?;
log::info!("PREV EOL: {byte}");
    loop {
        match byte {
            // CR VT FF
            0x0d => {
                let pos = bytes.pos();
                return Some(EOLMatch {
                    eol: EndOfLine::CR,
                    range: pos..pos + 1,
                });
            }
            0x0b => {
                let pos = bytes.pos();
                return Some(EOLMatch {
                    eol: EndOfLine::VT,
                    range: pos..pos + 1,
                });
            }
            0x0c => {
                let pos = bytes.pos();
                return Some(EOLMatch {
                    eol: EndOfLine::FF,
                    range: pos..pos + 1,
                });
            }
            // LF
            0x0a => {
                let prev = bytes.prev();
                let crlf = prev == Some(0x0d);
                if !crlf && prev.is_some() {
                    bytes.next();
                }

                let pos = bytes.pos();
                if crlf {
                    return Some(EOLMatch {
                        eol: EndOfLine::CRLF,
                        range: pos..pos + 2,
                    });
                } else {
                    return Some(EOLMatch {
                        eol: EndOfLine::LF,
                        range: pos..pos + 1,
                    });
                }
            }
            // NEL
            0x85 => {
                byte = bytes.prev()?;
                if byte != 0xc2 {
                    continue;
                }

                let pos = bytes.pos();
                return Some(EOLMatch {
                    eol: EndOfLine::NEL,
                    range: pos..pos + 2,
                });
            }
            // LS PS
            0xa8 => {
                byte = bytes.prev()?;
                if byte != 0x80 {
                    continue;
                }

                byte = bytes.prev()?;
                if byte != 0xe2 {
                    continue;
                }

                let pos = bytes.pos();
                return Some(EOLMatch {
                    eol: EndOfLine::PS,
                    range: pos..pos + 3,
                });
            }
            0xa9 => {
                byte = bytes.prev()?;
                if byte != 0x80 {
                    continue;
                }

                byte = bytes.prev()?;
                if byte != 0xe2 {
                    continue;
                }
                let pos = bytes.pos();
                return Some(EOLMatch {
                    eol: EndOfLine::LS,
                    range: pos..pos + 3,
                });
            }
            _ => {
                byte = bytes.prev()?;
            }
        }
    }
}

/// return position at line start of line
pub(crate) fn pos_at_line(slice: &PieceTreeSlice<'_>, line: u64) -> Option<u64> {
    let mut n = 0;
    let mut lines = slice.lines();

    while let Some(l) = lines.next() {
        if n == line {
            return Some(l.start());
        }

        n += 1;
    }

    None
}

/// return the line and its number at pos
pub(crate) fn line_at<'a>(slice: &PieceTreeSlice<'a>, pos: u64) -> (u64, PieceTreeSlice<'a>) {
    let mut lines = slice.lines();
    let mut cur = lines.next();
    let mut n = 0;

    while let Some(line) = cur {
        if line.range().contains(&pos) || (pos == slice.len() && pos == line.end()) {
            return (n, line);
        }

        n += 1;
        cur = lines.next();
    }

    panic!(
        "Tried get line at {pos} but slice length is: {}",
        slice.len()
    )
}

#[derive(Debug, Clone)]
pub struct Lines<'a> {
    bytes: Bytes<'a>,
    slice: PieceTreeSlice<'a>,
    at_end: bool,
}

impl<'a> Lines<'a> {
    #[inline]
    pub fn new(pt: &'a PieceTreeView, at: u64) -> Lines<'a> {
        let slice = pt.slice(..);
        let bytes = Bytes::new(pt, at);
        let mut lines = Lines {
            at_end: at == pt.len() && !pt.is_empty(),
            slice,
            bytes,
        };
        lines.goto_bol();
        lines
    }

    #[inline]
    pub fn new_from_slice(slice: &PieceTreeSlice<'a>, at: u64) -> Lines<'a> {
        let slice = slice.clone();
        let bytes = Bytes::new_from_slice(&slice, at);
        let mut lines = Lines {
            at_end: bytes.pos() == slice.len() && !slice.is_empty(),
            slice,
            bytes,
        };
        lines.goto_bol();
        lines
    }

    fn goto_bol(&mut self) {
        if self.bytes.pos() == self.slice.len() {
            return;
        }

        if let Some(m) = prev_eol(&mut self.bytes) {
            self.at_end = false;
            self.bytes.at(m.range.end);
        }
    }

    pub fn next(&mut self) -> Option<PieceTreeSlice<'a>> {
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

    pub fn prev(&mut self) -> Option<PieceTreeSlice<'a>> {
        let at_end = self.at_end;
        let end = self.bytes.pos();

        // Skip over previous eol
        if !at_end {
            prev_eol(&mut self.bytes);
        }
        self.at_end = false;

        match prev_eol(&mut self.bytes) {
            Some(mat) => {
                let start = mat.range.end;

                // Move bytes to start of line
                for _ in 0..mat.range.end - mat.range.start {
                    self.bytes.next();
                }

                Some(self.slice.slice(start..end))
            }
            None => {
                let start = self.bytes.pos();
                if start == end && !at_end {
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
    pub range: Range<u64>,
}

#[cfg(test)]
mod test {
    use crate::PieceTree;

    #[test]
    fn lines_next_empty() {
        let pt = PieceTree::new();
        let mut lines = pt.lines();
        assert_eq!(
            lines.next().as_ref().map(String::from),
            Some("".to_string())
        );

        assert_eq!(lines.next().as_ref().map(String::from), None);

        assert_eq!(
            lines.prev().as_ref().map(String::from),
            Some("".to_string())
        );

        assert_eq!(lines.prev().as_ref().map(String::from), None);

        assert_eq!(
            lines.next().as_ref().map(String::from),
            Some("".to_string())
        );
    }

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
