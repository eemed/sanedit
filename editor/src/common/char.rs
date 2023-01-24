use std::collections::HashMap;
use std::ops::Range;

use sanedit_buffer::piece_tree::PieceTreeSlice;

use smartstring::{LazyCompact, SmartString};
use unicode_width::UnicodeWidthStr;

/// Representation of a grapheme cluster (clusters of codepoints we treat as one
/// character) in the buffer.
/// This is a separate type to distinguish graphemes that have already been
/// converted to the format we want the user to see.
#[derive(Debug, Default, Clone, PartialEq, Hash)]
pub(crate) struct Char {
    display: SmartString<LazyCompact>,
    width: usize,
    buf_range: Option<Range<usize>>,
}

impl Char {
    pub fn new(grapheme: PieceTreeSlice, column: usize, options: &DisplayOptions) -> Char {
        grapheme_to_char(grapheme, column, options)
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn display(&self) -> &str {
        &self.display
    }
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
#[repr(usize)]
pub(crate) enum Replacement {
    Tab,
    TabFill,
    EOL,
    BufferEnd,
    TrailingWhitespace,
    NonBreakingSpace,
}

/// Options on how to display chars
#[derive(Debug, Clone)]
pub(crate) struct DisplayOptions {
    pub tabstop: usize,
    pub line_width: usize,
    pub replacements: HashMap<Replacement, String>,
}

impl Default for DisplayOptions {
    fn default() -> Self {
        use Replacement::*;

        let mut replacements = HashMap::new();
        replacements.insert(Tab, "→".to_string());
        replacements.insert(TabFill, " ".to_string());
        replacements.insert(EOL, "↲".to_string());
        replacements.insert(BufferEnd, "~".to_string());
        replacements.insert(TrailingWhitespace, "•".to_string());
        replacements.insert(NonBreakingSpace, "•".to_string());

        DisplayOptions {
            tabstop: 8,
            line_width: 80,
            replacements,
        }
    }
}

#[inline]
fn grapheme_to_char(grapheme: PieceTreeSlice, column: usize, options: &DisplayOptions) -> Char {
    let buf_range = Some(grapheme.start()..grapheme.end());
    // is tab
    if grapheme == "\t" {
        return tab_to_char(buf_range, column, options);
    }
    // TODO is eol
    // TODO is nbsp

    let display = {
        let mut display = SmartString::new();
        let mut chars = grapheme.chars();
        while let Some((_pos, _, ch)) = chars.next() {
            display.push(ch);
        }
        display
    };
    let width = UnicodeWidthStr::width(display.as_str()).max(1);

    Char {
        display,
        width,
        buf_range,
    }
}

fn tab_to_char(buf_range: Option<Range<usize>>, column: usize, options: &DisplayOptions) -> Char {
    // Calculate tab based on current visual column
    let width = options.tabstop - (column % options.tabstop);
    let first = options
        .replacements
        .get(&Replacement::Tab)
        .cloned()
        .unwrap_or_else(|| String::from(" "));
    let fill = options
        .replacements
        .get(&Replacement::TabFill)
        .cloned()
        .unwrap_or_else(|| String::from(" "));

    let mut display: SmartString<LazyCompact> = first.into();
    for _ in 1..width {
        display.push_str(&fill);
    }

    Char {
        display,
        width,
        buf_range,
    }
}

#[cfg(test)]
mod test {
    use sanedit_buffer::piece_tree::PieceTree;

    use super::*;

    #[test]
    fn emoji() {
        let mut pt = PieceTree::new();
        pt.insert_str(0, "❤️");
        let slice = pt.slice(..);
        let ch = Char::new(slice, 0, &DisplayOptions::default());
        assert_eq!("❤️", ch.display());
    }

    #[test]
    fn control_sequence_null() {
        let mut pt = PieceTree::new();
        pt.insert(0, b"\0");
        let slice = pt.slice(..);
        let ch = Char::new(slice, 0, &DisplayOptions::default());
        assert_eq!("\0", ch.display());
    }

    #[test]
    fn invalid_utf8() {
        let mut pt = PieceTree::new();
        pt.insert(0, b"\xFF");
        let slice = pt.slice(..);
        let ch = Char::new(slice, 0, &DisplayOptions::default());
        assert_eq!("\u{fffd}", ch.display());
    }

    #[test]
    fn tab() {
        let mut pt = PieceTree::new();
        pt.insert_str(0, "\t");
        let slice = pt.slice(..);
        let opts = DisplayOptions::default();
        let expected = {
            let mut first = opts.replacements.get(&Replacement::Tab).unwrap().clone();
            let fill = opts.replacements.get(&Replacement::TabFill).unwrap();
            for _ in 0..7 {
                first.push_str(fill);
            }
            first
        };
        let ch = Char::new(slice, 0, &DisplayOptions::default());
        assert_eq!(&expected, ch.display());
    }

    #[test]
    fn non_breaking_space() {
        let mut pt = PieceTree::new();
        pt.insert_str(0, "\u{00A0}");
        let slice = pt.slice(..);
        let opts = DisplayOptions::default();
        let ch = Char::new(slice, 0, &opts);
        let expected = opts
            .replacements
            .get(&Replacement::NonBreakingSpace)
            .unwrap();
        assert_eq!(expected, ch.display());
    }
}
