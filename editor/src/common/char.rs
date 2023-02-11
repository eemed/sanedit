use std::collections::HashMap;
use std::ops::Range;

use sanedit_buffer::piece_tree::PieceTreeSlice;

use unicode_width::UnicodeWidthStr;

use super::eol::EOL;

/// Representation of a grapheme cluster (clusters of codepoints we treat as one
/// character) in the buffer.
/// This is a separate type to distinguish graphemes that have already been
/// converted to the format we want the user to see.
#[derive(Debug, Default, Clone, PartialEq, Hash)]
pub(crate) struct Char {
    display: Option<String>,
    width: usize,
    grapheme_range: Option<Range<usize>>,
    grapheme: String,
}

impl Char {
    pub fn new(grapheme: PieceTreeSlice, column: usize, options: &DisplayOptions) -> Char {
        grapheme_to_char(grapheme, column, options)
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn display(&self) -> &str {
        self.display.as_ref().unwrap_or(&self.grapheme)
    }

    pub fn grapheme_len(&self) -> usize {
        self.grapheme_range
            .as_ref()
            .map(|range| range.len())
            .unwrap_or(0)
    }

    pub fn grapheme_category(&self) -> GraphemeCategory {
        grapheme_category(&self.grapheme)
    }
}

#[derive(PartialEq, Default, Clone, Copy, Debug, Hash)]
pub(crate) enum GraphemeCategory {
    EOL,
    Whitespace,
    Word,
    Punctuation,

    #[default]
    Unknown,
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
    pub theme: String,
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
            theme: "gruvbox".into(),
            tabstop: 8,
            line_width: 80,
            replacements,
        }
    }
}

#[inline]
fn grapheme_to_char(slice: PieceTreeSlice, column: usize, options: &DisplayOptions) -> Char {
    let buf_range = Some(slice.start()..slice.end());
    let grapheme = String::from(&slice);

    // is tab
    if grapheme == "\t" {
        return tab_to_char(grapheme, buf_range, column, options);
    }
    // is eol
    if EOL::is_eol_bytes(&grapheme) {
        return eol_to_char(grapheme, buf_range, options);
    }
    // TODO is nbsp

    let width = UnicodeWidthStr::width(grapheme.as_str()).max(1);

    Char {
        display: None,
        width,
        grapheme_range: buf_range,
        grapheme,
    }
}

fn tab_to_char(
    grapheme: String,
    buf_range: Option<Range<usize>>,
    column: usize,
    options: &DisplayOptions,
) -> Char {
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

    let mut display: String = first;
    for _ in 1..width {
        display.push_str(&fill);
    }

    Char {
        display: Some(display),
        width,
        grapheme_range: buf_range,
        grapheme,
    }
}

fn eol_to_char(
    grapheme: String,
    buf_range: Option<Range<usize>>,
    options: &DisplayOptions,
) -> Char {
    let display = options
        .replacements
        .get(&Replacement::EOL)
        .cloned()
        .unwrap_or_else(|| String::from(" "));
    let width = display.width();

    Char {
        display: Some(display),
        width,
        grapheme_range: buf_range,
        grapheme,
    }
}

#[inline(always)]
pub(crate) fn grapheme_category(grapheme: &str) -> GraphemeCategory {
    if EOL::is_eol_bytes(grapheme) {
        return GraphemeCategory::EOL;
    }

    if grapheme
        .chars()
        .fold(true, |acc, ch| acc && ch.is_whitespace())
    {
        return GraphemeCategory::Whitespace;
    }

    if grapheme
        .chars()
        .fold(true, |acc, ch| acc && (ch.is_alphanumeric() || ch == '_'))
    {
        return GraphemeCategory::Word;
    }

    if grapheme.chars().count() == 1 {
        let char = grapheme.chars().next().unwrap();
        if char.is_ascii() && !char.is_alphanumeric() {
            return GraphemeCategory::Punctuation;
        }
    }

    GraphemeCategory::Unknown
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
