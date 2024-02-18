use std::collections::HashMap;
use std::ops::Range;

use sanedit_buffer::PieceTreeSlice;

use sanedit_buffer::utf8::EndOfLine;
use smallvec::SmallVec;
use unicode_width::UnicodeWidthStr;

/// Representation of a grapheme cluster (clusters of codepoints we treat as one
/// character) in the buffer.
/// This is a separate type to distinguish graphemes that have already been
/// converted to the format we want the user to see.
#[derive(Debug, Default, Clone, PartialEq, Hash)]
pub(crate) struct Char {
    /// The displayed thing
    display: String,

    /// Width of the displayed string
    width: usize,

    /// Length of the thing we are displaying in buffer
    len_in_buffer: usize,
}

impl Char {
    pub fn new(grapheme: &PieceTreeSlice, column: usize, options: &DisplayOptions) -> Char {
        grapheme_to_char(grapheme, column, options)
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn display(&self) -> &str {
        &self.display
    }

    pub fn len_in_buffer(&self) -> usize {
        self.len_in_buffer
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
    #[inline(always)]
    pub fn is_word(&self) -> bool {
        use GraphemeCategory::*;
        matches!(self, Word | Punctuation)
    }
}

#[inline(always)]
pub(crate) fn is_word_break(prev: &GraphemeCategory, next: &GraphemeCategory) -> bool {
    prev != next && next.is_word()
}

#[inline(always)]
pub(crate) fn is_word_break_end(prev: &GraphemeCategory, next: &GraphemeCategory) -> bool {
    prev != next && prev.is_word()
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
    pub replacements: HashMap<Replacement, String>,
}

impl Default for DisplayOptions {
    fn default() -> Self {
        use Replacement::*;

        const DEFAULT: [(Replacement, &'static str); 6] = [
            (Tab, "→"),
            (TabFill, " "),
            (EOL, " "),
            (BufferEnd, "~"),
            (TrailingWhitespace, "•"),
            (NonBreakingSpace, "•"),
        ];

        let mut replacements = HashMap::new();
        for (rep, s) in DEFAULT {
            replacements.insert(rep, s.into());
        }

        DisplayOptions {
            theme: "default".into(),
            tabstop: 8,
            replacements,
        }
    }
}

#[inline]
fn grapheme_to_char(slice: &PieceTreeSlice, column: usize, options: &DisplayOptions) -> Char {
    let blen = slice.len();
    let grapheme = String::from(slice);

    // is tab
    if grapheme == "\t" {
        return tab_to_char(blen, column, options);
    }
    // is eol
    if EndOfLine::is_eol(&grapheme) {
        return eol_to_char(blen, options);
    }

    // is nbsp
    if grapheme == "\u{00A0}" {
        return nbsp_to_char(blen, options);
    }

    let single_char = grapheme.chars().count() == 1;
    if single_char {
        let ch = grapheme.chars().next().unwrap();
        if ch.is_control() {
            return control_to_char(grapheme, blen);
        }
    }

    let width = grapheme.width().max(1);

    Char {
        display: grapheme,
        width,
        len_in_buffer: blen,
    }
}

fn tab_to_char(blen: usize, column: usize, options: &DisplayOptions) -> Char {
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
        display,
        width,
        len_in_buffer: blen,
    }
}

fn eol_to_char(blen: usize, options: &DisplayOptions) -> Char {
    let display = options
        .replacements
        .get(&Replacement::EOL)
        .cloned()
        .unwrap_or_else(|| String::from(" "));
    let width = display.width();

    Char {
        display,
        width,
        len_in_buffer: blen,
    }
}

fn nbsp_to_char(blen: usize, options: &DisplayOptions) -> Char {
    let display = options
        .replacements
        .get(&Replacement::NonBreakingSpace)
        .cloned()
        .unwrap_or_else(|| String::from(" "));
    let width = display.width();

    Char {
        display,
        width,
        len_in_buffer: blen,
    }
}

fn control_to_char(grapheme: String, blen: usize) -> Char {
    let ch = grapheme.chars().next().unwrap();
    // C0 control codes or ascii control codes
    if ch.is_ascii_control() {
        return ascii_control_to_char(grapheme, blen).unwrap();
    }

    // C1 control codes, unicode control codes of form [0xc2, xx]
    let hex: String = format!("<{:02x}>", grapheme.as_bytes()[1]);
    let width = hex.width();

    Char {
        display: hex.into(),
        width,
        len_in_buffer: blen,
    }
}

fn ascii_control_to_char(grapheme: String, blen: usize) -> Option<Char> {
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
        display: rep.into(),
        width,
        len_in_buffer: blen,
    })
}

#[inline(always)]
pub(crate) fn grapheme_category(grapheme: &PieceTreeSlice) -> GraphemeCategory {
    let chars = {
        // read chars to a buf for easier handling
        let mut chars: SmallVec<[char; 4]> = smallvec::SmallVec::new();
        let mut iter = grapheme.chars();
        while let Some((_, _, ch)) = iter.next() {
            chars.push(ch);
        }
        chars
    };

    if chars.iter().all(|ch| ch.is_alphanumeric() || *ch == '_') {
        return GraphemeCategory::Word;
    }

    if chars.iter().all(|ch| ch.is_whitespace()) {
        return GraphemeCategory::Whitespace;
    }

    if chars.len() == 1 {
        let ch = chars[0];
        if ch.is_ascii() && !ch.is_alphanumeric() {
            return GraphemeCategory::Punctuation;
        }

        if ch.is_control() {
            return GraphemeCategory::ControlCode;
        }
    }

    if EndOfLine::is_slice_eol(grapheme) {
        return GraphemeCategory::EOL;
    }

    GraphemeCategory::Unknown
}

#[cfg(test)]
mod test {
    use sanedit_buffer::PieceTree;

    use super::*;

    #[test]
    fn emoji() {
        let mut pt = PieceTree::new();
        pt.insert(0, "❤️");
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
        pt.insert(0, "\t");
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
        pt.insert(0, "\u{00A0}");
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
