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
    pub fn new(grapheme: &PieceTreeSlice, column: usize, options: &DisplayOptions) -> Char {
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

    pub fn grapheme(&self) -> &str {
        &self.grapheme
    }
}

#[derive(PartialEq, Default, Clone, Copy, Debug, Hash)]
pub(crate) enum GraphemeCategory {
    EOL,
    Whitespace,
    Word,
    Punctuation,
    ControlCode,

    #[default]
    Unknown,
}

impl GraphemeCategory {
    pub fn is_word_break(&self, prev: &GraphemeCategory) -> bool {
        use GraphemeCategory::*;
        prev != self && matches!(self, Word | Punctuation)
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
    pub theme: String,
    pub tabstop: usize,
    pub line_width: usize,
    pub replacements: HashMap<Replacement, String>,
}

impl Default for DisplayOptions {
    fn default() -> Self {
        use Replacement::*;

        let mut replacements = HashMap::new();
        replacements.insert(Tab, "→".into());
        replacements.insert(TabFill, " ".into());
        replacements.insert(EOL, " ".into());
        replacements.insert(BufferEnd, "~".into());
        replacements.insert(TrailingWhitespace, "•".into());
        replacements.insert(NonBreakingSpace, "•".into());

        DisplayOptions {
            theme: "gruvbox".into(),
            tabstop: 8,
            line_width: 80,
            replacements,
        }
    }
}

#[inline]
fn grapheme_to_char(slice: &PieceTreeSlice, column: usize, options: &DisplayOptions) -> Char {
    let buf_range = Some(slice.start()..slice.end());
    let mut grapheme = String::from(slice);

    // is tab
    if grapheme == "\t" {
        return tab_to_char(grapheme, buf_range, column, options);
    }
    // is eol
    if EOL::is_eol_bytes(&grapheme) {
        return eol_to_char(grapheme, buf_range, options);
    }

    // is nbsp
    if grapheme == "\u{00A0}" {
        return nbsp_to_char(grapheme, buf_range, options);
    }

    let single_char = grapheme.chars().count() == 1;
    if single_char {
        let ch = grapheme.chars().next().unwrap();
        if ch.is_control() {
            return control_to_char(grapheme, buf_range);
        }
    }

    let width = grapheme.width().max(1);

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

fn nbsp_to_char(
    grapheme: String,
    buf_range: Option<Range<usize>>,
    options: &DisplayOptions,
) -> Char {
    let display = options
        .replacements
        .get(&Replacement::NonBreakingSpace)
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

fn control_to_char(grapheme: String, buf_range: Option<Range<usize>>) -> Char {
    let ch = grapheme.chars().next().unwrap();
    // C0 control codes or ascii control codes
    if ch.is_ascii_control() {
        return ascii_control_to_char(grapheme, buf_range).unwrap();
    }

    // C1 control codes, unicode control codes of form [0xc2, xx]
    let hex: String = format!("<{:02x}>", grapheme.as_bytes()[1]);
    let width = hex.width();

    Char {
        display: Some(hex.into()),
        width,
        grapheme_range: buf_range,
        grapheme: grapheme.into(),
    }
}

fn ascii_control_to_char(grapheme: String, buf_range: Option<Range<usize>>) -> Option<Char> {
    let byte = grapheme.bytes().next()?;
    let rep = match byte {
        0 => "^@",
        1 => "^A",
        2 => "^B",
        3 => "^C",
        4 => "^D",
        5 => "^E",
        6 => "^F",
        7 => "^G",
        8 => "^H",
        9 => "^I",
        10 => "^J",
        11 => "^K",
        12 => "^L",
        13 => "^M",
        14 => "^N",
        15 => "^O",
        16 => "^P",
        17 => "^Q",
        18 => "^R",
        19 => "^S",
        20 => "^T",
        21 => "^U",
        22 => "^V",
        23 => "^W",
        24 => "^X",
        25 => "^Y",
        26 => "^Z",
        27 => "^[",
        28 => "^\\",
        29 => "^]",
        30 => "^^",
        31 => "^_",
        127 => "^?",
        _ => unreachable!("non ascii control char"),
    };

    let width = rep.width();

    Some(Char {
        display: Some(rep.into()),
        width,
        grapheme_range: buf_range,
        grapheme: grapheme.into(),
    })
}

#[inline(always)]
pub(crate) fn grapheme_category(grapheme: &str) -> GraphemeCategory {
    if grapheme.chars().all(|ch| ch.is_alphanumeric() || ch == '_') {
        return GraphemeCategory::Word;
    }

    if grapheme.chars().all(|ch| ch.is_whitespace()) {
        return GraphemeCategory::Whitespace;
    }

    if grapheme.chars().count() == 1 {
        let ch = grapheme.chars().next().unwrap();
        if ch.is_ascii() && !ch.is_alphanumeric() {
            return GraphemeCategory::Punctuation;
        }

        if ch.is_control() {
            return GraphemeCategory::ControlCode;
        }
    }

    if EOL::is_eol_bytes(grapheme) {
        return GraphemeCategory::EOL;
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
        let ch = Char::new(&slice, 0, &DisplayOptions::default());
        assert_eq!("❤️", ch.display());
    }

    #[test]
    fn control_sequence_null() {
        let mut pt = PieceTree::new();
        pt.insert(0, b"\0");
        let slice = pt.slice(..);
        let ch = Char::new(&slice, 0, &DisplayOptions::default());
        assert_eq!("^@", ch.display());
    }

    #[test]
    fn invalid_utf8() {
        let mut pt = PieceTree::new();
        pt.insert(0, b"\xFF");
        let slice = pt.slice(..);
        let ch = Char::new(&slice, 0, &DisplayOptions::default());
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
        let ch = Char::new(&slice, 0, &DisplayOptions::default());
        assert_eq!(&expected, ch.display());
    }

    #[test]
    fn non_breaking_space() {
        let mut pt = PieceTree::new();
        pt.insert_str(0, "\u{00A0}");
        let slice = pt.slice(..);
        let opts = DisplayOptions::default();
        let ch = Char::new(&slice, 0, &opts);
        let expected = opts
            .replacements
            .get(&Replacement::NonBreakingSpace)
            .unwrap();
        assert_eq!(expected, ch.display());
    }
}
