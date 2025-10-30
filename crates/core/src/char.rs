use rustc_hash::FxHashMap;

use sanedit_buffer::utf8::{decode_utf8, EndOfLine, Grapheme};
use sanedit_utils::either::Either;
use unicode_width::UnicodeWidthStr;

use self::flags::Flags;

/// Representation of a grapheme cluster (clusters of codepoints we treat as one
/// character) in the buffer.
/// This is a separate type to distinguish graphemes that have already been
/// converted to the format we want the user to see.
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum Chars {
    Single { ch: Char },
    Multi { chars: Vec<Char> },
}

impl Default for Chars {
    fn default() -> Self {
        Chars::Single {
            ch: Char::default(),
        }
    }
}

impl Chars {
    pub fn new(grapheme: &Grapheme, column: usize, options: &DisplayOptions) -> Chars {
        grapheme_to_char(grapheme, column, options)
    }

    fn wide(grapheme: String, len: u64) -> Chars {
        let ch = Char {
            character: ' ',
            extra: Some(Box::new(CharExtra { wide: grapheme })),
            flags: flags::NONE,
            len_in_buffer: len,
        };

        Chars::Single { ch }
    }

    fn from_str(string: &str, len: u64) -> Chars {
        let mut chars = vec![];
        for ch in string.chars() {
            let ch2 = Char {
                character: ch,
                extra: None,
                flags: if chars.is_empty() {
                    flags::PLACE_CURSOR | flags::VIRTUAL
                } else {
                    flags::VIRTUAL
                },
                len_in_buffer: if chars.is_empty() { len } else { 0 },
            };

            chars.push(ch2);
        }

        Chars::Multi { chars }
    }

    pub fn len(&self) -> usize {
        match self {
            Chars::Single { .. } => 1,
            Chars::Multi { chars } => chars.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn width(&self) -> usize {
        match self {
            Chars::Single { ch } => ch.width(),
            Chars::Multi { chars } => chars.iter().map(Char::width).sum(),
        }
    }

    pub fn len_in_buffer(&self) -> u64 {
        match self {
            Chars::Single { ch } => ch.len_in_buffer(),
            Chars::Multi { chars } => chars.iter().map(Char::len_in_buffer).sum(),
        }
    }

    pub fn is_eol(&self) -> bool {
        match self {
            Chars::Single { ch } => ch.is_eol(),
            Chars::Multi { chars } => chars.iter().any(Char::is_eol),
        }
    }
}

impl From<Char> for Chars {
    fn from(value: Char) -> Self {
        Chars::Single { ch: value }
    }
}

mod flags {
    pub(crate) type Flags = u8;

    pub(crate) const NONE: u8 = 0;

    /// Just keep eol status so we can fetch it whenever
    pub(crate) const EOL: u8 = 1;

    /// The character does not exist in the buffer at all or is a representation of it
    pub(crate) const VIRTUAL: u8 = 1 << 1;

    /// Can place cursor on this block
    pub(crate) const PLACE_CURSOR: u8 = 1 << 2;
}

#[derive(Debug, Clone, PartialEq, Hash, Default)]
pub struct CharExtra {
    wide: String,
}

#[derive(Debug, Default, Clone, PartialEq, Hash)]
pub struct Char {
    character: char,
    extra: Option<Box<CharExtra>>,
    flags: Flags,

    /// Length of the thing we are displaying in buffer
    len_in_buffer: u64,
}

impl Char {
    pub fn new_virtual(ch: char) -> Char {
        Char {
            character: ch,
            extra: None,
            flags: flags::VIRTUAL,
            len_in_buffer: 0,
        }
    }

    pub fn width(&self) -> usize {
        let width = if let Some(extra) = self.extra.as_ref() {
            extra.wide.width()
        } else {
            let mut buf = [0u8; 4];
            let ch = self.character.encode_utf8(&mut buf);
            ch.width()
        };

        width.max(1)
    }

    pub fn display(&self) -> Either<&'_ str, char> {
        if let Some(extra) = self.extra.as_ref() {
            Either::Left(extra.wide.as_str())
        } else {
            Either::Right(self.character)
        }
    }

    pub fn len_in_buffer(&self) -> u64 {
        self.len_in_buffer
    }

    pub fn is_eol(&self) -> bool {
        self.flags & flags::EOL == flags::EOL
    }

    pub fn is_virtual(&self) -> bool {
        self.flags & flags::VIRTUAL == flags::VIRTUAL
    }

    pub fn can_place_cursor(&self) -> bool {
        if self.is_virtual() {
            self.flags & flags::PLACE_CURSOR == flags::PLACE_CURSOR
        } else {
            true
        }
    }
}

#[derive(PartialEq, Default, Clone, Copy, Debug, Hash)]
pub enum GraphemeCategory {
    Eol,
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
        matches!(self, Word | Punctuation | Eol)
    }
}

#[inline(always)]
pub fn is_word_break(prev: &GraphemeCategory, next: &GraphemeCategory) -> bool {
    prev != next && next.is_word()
}

#[inline(always)]
pub fn is_word_break_end(prev: &GraphemeCategory, next: &GraphemeCategory) -> bool {
    prev != next && prev.is_word()
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
#[repr(usize)]
pub enum Replacement {
    Tab,
    TabFill,
    EOL,
    BufferEnd,
    TrailingWhitespace,
    NonBreakingSpace,
    Wrap,
}

/// Options on how to display chars
#[derive(Debug, Clone)]
pub struct DisplayOptions {
    pub width: usize,
    pub height: usize,
    pub tabstop: u8,
    pub replacements: FxHashMap<Replacement, char>,
}

impl Default for DisplayOptions {
    fn default() -> Self {
        Self::new(80, 24)
    }
}

impl DisplayOptions {
    pub fn new(width: usize, height: usize) -> DisplayOptions {
        use Replacement::*;

        const DEFAULT: [(Replacement, char); 7] = [
            (Tab, '›'),
            (TabFill, ' '),
            (EOL, ' '),
            (BufferEnd, '~'),
            (TrailingWhitespace, '•'),
            (NonBreakingSpace, '•'),
            (Wrap, '↳'),
        ];

        let mut replacements = FxHashMap::default();
        for (rep, s) in DEFAULT {
            replacements.insert(rep, s);
        }

        DisplayOptions {
            width,
            height,
            tabstop: 8,
            replacements,
        }
    }

    pub fn wrap_char_width(&self) -> usize {
        self.replacements
            .get(&Replacement::Wrap)
            .map(|rep| Char::new_virtual(*rep).width())
            .unwrap_or(0)
    }
}

fn grapheme_to_char(grapheme: &Grapheme, column: usize, options: &DisplayOptions) -> Chars {
    let blen = grapheme.len();
    let bytes = grapheme.as_ref();
    let (ch, n) = decode_utf8(bytes);
    let ch = if n == bytes.len() { ch } else { None };

    if let Some(ch) = ch {
        let is_byte = ch.len_utf8() == 1;

        if is_byte {
            let mut buf = [0u8; 1];
            let _ = ch.encode_utf8(&mut buf);
            let byte = buf[0];

            if ch.is_ascii_alphanumeric() {
                return Char {
                    character: ch,
                    extra: None,
                    flags: Flags::default(),
                    len_in_buffer: blen,
                }
                .into();
            }

            // is tab
            if ch == '\t' {
                return tab_to_char(blen, column, options);
            }

            if EndOfLine::is_byte_eol(byte) {
                return eol_to_char(blen, options).into();
            }
        }

        if EndOfLine::is_eol_char(ch) {
            return eol_to_char(blen, options).into();
        }

        // is nbsp
        if ch == '\u{00A0}' {
            return nbsp_to_char(blen, options).into();
        }

        if ch.is_control() {
            return control_to_char(ch.to_string(), blen);
        }

        Char {
            character: ch,
            extra: None,
            flags: Flags::default(),
            len_in_buffer: blen,
        }
        .into()
    } else {
        if grapheme.is_eol() {
            return eol_to_char(blen, options).into();
        }

        Chars::wide(grapheme.to_string(), blen)
    }
}

fn tab_to_char(blen: u64, column: usize, options: &DisplayOptions) -> Chars {
    // Calculate tab based on current visual column
    let width = options.tabstop as usize - (column % options.tabstop as usize);
    let first = options
        .replacements
        .get(&Replacement::Tab)
        .cloned()
        .unwrap_or(' ');
    let fill = options
        .replacements
        .get(&Replacement::TabFill)
        .cloned()
        .unwrap_or(' ')
        .to_string();
    let tab = format!("{}{}", first, fill.repeat(width - 1));
    Chars::from_str(&tab, blen)
}

fn eol_to_char(blen: u64, options: &DisplayOptions) -> Char {
    let repl = options.replacements.get(&Replacement::EOL);
    let character = repl.cloned().unwrap_or(' ');

    Char {
        character,
        extra: None,
        flags: flags::EOL,
        len_in_buffer: blen,
    }
}

fn nbsp_to_char(blen: u64, options: &DisplayOptions) -> Char {
    let repl = options.replacements.get(&Replacement::NonBreakingSpace);
    let character = repl.cloned().unwrap_or(' ');

    Char {
        character,
        extra: None,
        flags: flags::NONE,
        len_in_buffer: blen,
    }
}

fn control_to_char(grapheme: String, blen: u64) -> Chars {
    let ch = grapheme.chars().next().unwrap();
    // C0 control codes or ascii control codes
    if ch.is_ascii_control() {
        return ascii_control_to_char(ch as u8);
    }

    // C1 control codes, unicode control codes of form [0xc2, xx]
    let hex: String = format!("<{:02x}>", grapheme.as_bytes()[1]);
    Chars::from_str(&hex, blen)
}

fn ascii_control_to_char(byte: u8) -> Chars {
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

    Chars::from_str(rep, 1)
}

pub fn grapheme_category(grapheme: &Grapheme) -> GraphemeCategory {
    if grapheme.is_eol() {
        return GraphemeCategory::Eol;
    }

    let bytes = grapheme.as_ref();
    // let (mut ch, mut n) = decode_utf8(bytes);
    let (ch, _n) = decode_utf8(bytes);

    // if n == bytes.len() {
    let ch = ch.unwrap_or('\u{fffd}');

    if ch.is_whitespace() {
        return GraphemeCategory::Whitespace;
    }

    if ch.is_alphanumeric() || ch == '_' {
        return GraphemeCategory::Word;
    }

    if ch.is_ascii() && !ch.is_alphanumeric() {
        return GraphemeCategory::Punctuation;
    }

    if ch.is_control() {
        return GraphemeCategory::ControlCode;
    }

    GraphemeCategory::Unknown
}

#[cfg(test)]
mod test {
    use sanedit_buffer::PieceTree;

    use super::*;

    impl Chars {
        pub fn display(&self) -> String {
            let mut result = String::new();
            match self {
                Chars::Single { ch } => match ch.display() {
                    Either::Left(s) => result.push_str(s),
                    Either::Right(ch) => result.push(ch),
                },
                Chars::Multi { chars } => {
                    for ch in chars {
                        match ch.display() {
                            Either::Left(s) => result.push_str(s),
                            Either::Right(ch) => result.push(ch),
                        }
                    }
                }
            }
            result
        }
    }

    #[test]
    fn emoji() {
        let mut pt = PieceTree::new();
        pt.insert(0, "❤️");
        let slice = pt.slice(..);
        let g = slice.graphemes().next().unwrap();
        let ch = Chars::new(&g, 0, &DisplayOptions::default());
        assert_eq!("❤️", ch.display());
    }

    #[test]
    fn control_sequence_null() {
        let mut pt = PieceTree::new();
        pt.insert(0, b"\0");
        let slice = pt.slice(..);
        let g = slice.graphemes().next().unwrap();
        let ch = Chars::new(&g, 0, &DisplayOptions::default());
        assert_eq!("^@", ch.display());
    }

    #[test]
    fn invalid_utf8() {
        let mut pt = PieceTree::new();
        pt.insert(0, b"\xFF");
        let slice = pt.slice(..);
        let g = slice.graphemes().next().unwrap();
        let ch = Chars::new(&g, 0, &DisplayOptions::default());
        assert_eq!("\u{fffd}", ch.display());
    }

    #[test]
    fn tab() {
        let mut pt = PieceTree::new();
        pt.insert(0, "\t");
        let slice = pt.slice(..);
        let g = slice.graphemes().next().unwrap();
        let opts = DisplayOptions::default();
        let expected = {
            let mut first = opts
                .replacements
                .get(&Replacement::Tab)
                .unwrap()
                .to_string();
            let fill = opts.replacements.get(&Replacement::TabFill).unwrap();
            for _ in 0..7 {
                first.push(*fill);
            }
            first
        };
        let ch = Chars::new(&g, 0, &DisplayOptions::default());
        assert_eq!(expected, ch.display());
    }

    #[test]
    fn non_breaking_space() {
        let mut pt = PieceTree::new();
        pt.insert(0, "\u{00A0}");

        let slice = pt.slice(..);
        let g = slice.graphemes().next().unwrap();
        let opts = DisplayOptions::default();
        let ch = Chars::new(&g, 0, &opts);
        let expected = opts
            .replacements
            .get(&Replacement::NonBreakingSpace)
            .unwrap()
            .to_string();
        assert_eq!(expected, ch.display());
    }
}
