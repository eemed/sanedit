use std::borrow::Cow;

use rustc_hash::FxHashMap;
use sanedit_buffer::PieceTreeSlice;

use sanedit_buffer::utf8::EndOfLine;
use unicode_width::UnicodeWidthStr;

use self::flags::Flags;

/// Representation of a grapheme cluster (clusters of codepoints we treat as one
/// character) in the buffer.
/// This is a separate type to distinguish graphemes that have already been
/// converted to the format we want the user to see.
#[derive(Debug, Default, Clone, PartialEq, Hash)]
pub struct Chars {
    chars: Vec<Char>,
}

impl Chars {
    pub fn new(grapheme: &PieceTreeSlice, column: usize, options: &DisplayOptions) -> Chars {
        grapheme_to_char(grapheme, column, options)
    }

    fn wide(grapheme: String, len: u64) -> Chars {
        let fill = grapheme.width().max(1) - 1;
        let mut chars = vec![Char {
            character: ' ',
            extra: Some(Box::new(CharExtra { wide: grapheme })),
            flags: flags::WIDE | flags::CONTINUE_START,
            len_in_buffer: len,
        }];

        for _ in 0..fill {
            chars.push(Char {
                character: ' ',
                extra: None,
                flags: flags::NO_WIDTH | flags::CONTINUE,
                len_in_buffer: 0,
            });
        }

        Chars { chars }
    }

    fn from_str_virtual(string: &str, len: u64) -> Chars {
        let mut chars = vec![];
        for ch in string.chars() {
            let ch2 = if chars.is_empty() {
                Char {
                    character: ch,
                    extra: None,
                    flags: flags::VIRTUAL | flags::CONTINUE_START,
                    len_in_buffer: len,
                }
            } else {
                Char {
                    character: ch,
                    extra: None,
                    flags: flags::VIRTUAL | flags::CONTINUE,
                    len_in_buffer: 0,
                }
            };

            chars.push(ch2);
        }

        Chars { chars }
    }

    pub fn len(&self) -> usize {
        self.chars.len()
    }

    pub fn width(&self) -> usize {
        self.chars.iter().map(Char::width).sum()
    }

    pub fn len_in_buffer(&self) -> u64 {
        self.chars.iter().map(Char::len_in_buffer).sum()
    }

    pub fn is_eol(&self) -> bool {
        self.chars.iter().any(Char::is_eol)
    }

    pub fn display(&self) -> String {
        let mut result = String::new();
        for ch in &self.chars {
            result.push_str(ch.display().as_ref());
        }

        result
    }
}

impl From<Char> for Chars {
    fn from(value: Char) -> Self {
        Chars { chars: vec![value] }
    }
}

impl From<Chars> for Vec<Char> {
    fn from(value: Chars) -> Self {
        value.chars
    }
}

mod flags {
    pub(crate) type Flags = u8;

    /// This char continues the previous one, for example Tab + tab fills
    pub(crate) const CONTINUE: u8 = 1 << 0;

    /// Grapheme is not one char, something like zerowidth joiner used to merge
    /// multiple together
    pub(crate) const WIDE: u8 = 1 << 1;

    /// This char has no width, used as padding with wide chars
    pub(crate) const NO_WIDTH: u8 = 1 << 2;

    /// Just keep eol status so we can fetch it whenever
    pub(crate) const EOL: u8 = 1 << 3;

    /// The display character is different than the actual character in buffer
    /// or it may not exist at all in the buffer
    pub(crate) const VIRTUAL: u8 = 1 << 4;

    /// This char starts the continue block
    pub(crate) const CONTINUE_START: u8 = 1 << 5;
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
        if self.flags & flags::NO_WIDTH != 0 {
            return 0;
        }

        let width = if self.flags & flags::WIDE != 0 {
            let extra = self.extra.as_ref().unwrap();
            extra.wide.width()
        } else {
            let mut buf = [0u8; 4];
            let ch = self.character.encode_utf8(&mut buf);
            ch.width()
        };

        width.max(1)
    }

    pub fn display(&self) -> Cow<'_, str> {
        if self.flags & flags::WIDE != 0 {
            let extra = self.extra.as_ref().unwrap();
            Cow::Borrowed(extra.wide.as_str())
        } else {
            Cow::Owned(self.character.to_string())
        }
    }

    pub fn len_in_buffer(&self) -> u64 {
        self.len_in_buffer
    }

    pub fn is_eol(&self) -> bool {
        self.flags & flags::EOL != 0
    }

    pub fn is_continue(&self) -> bool {
        self.flags & flags::CONTINUE != 0
    }

    pub fn is_continue_start(&self) -> bool {
        self.flags & flags::CONTINUE_START != 0
    }

    pub fn is_virtual(&self) -> bool {
        self.flags & flags::VIRTUAL != 0
    }
}

#[derive(PartialEq, Default, Clone, Copy, Debug, Hash)]
pub enum GraphemeCategory {
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
        matches!(self, Word | Punctuation | EOL)
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
    pub tabstop: u32,
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
            (Tab, '→'),
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

#[inline]
fn grapheme_to_char(slice: &PieceTreeSlice, column: usize, options: &DisplayOptions) -> Chars {
    let blen = slice.len();
    let chars: Vec<char> = {
        let mut all = vec![];
        let mut chars = slice.chars();
        while let Some((_, _, ch)) = chars.next() {
            all.push(ch);
        }
        all
    };
    let single_char = chars.len() == 1;

    if single_char {
        let ch = chars[0];
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
            character: chars[0],
            extra: None,
            flags: Flags::default(),
            len_in_buffer: blen,
        }
        .into()
    } else {
        let grapheme = String::from(slice);
        if EndOfLine::is_eol(&grapheme) {
            return eol_to_char(blen, options).into();
        }

        Chars::wide(grapheme, blen)
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
    Chars::from_str_virtual(&tab, blen)
}

fn eol_to_char(blen: u64, options: &DisplayOptions) -> Char {
    let repl = options.replacements.get(&Replacement::EOL);
    let character = repl.cloned().unwrap_or(' ');

    let mut flags = flags::EOL;
    if repl.is_some() {
        flags |= flags::VIRTUAL;
    }

    Char {
        character,
        extra: None,
        flags,
        len_in_buffer: blen,
    }
}

fn nbsp_to_char(blen: u64, options: &DisplayOptions) -> Char {
    let repl = options.replacements.get(&Replacement::NonBreakingSpace);
    let character = repl.cloned().unwrap_or(' ');

    let mut flags = flags::EOL;
    if repl.is_some() {
        flags |= flags::VIRTUAL;
    }

    Char {
        character,
        extra: None,
        flags,
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
    Chars::from_str_virtual(&hex, blen)
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

    Chars::from_str_virtual(rep, 1)
}

pub fn grapheme_category(grapheme: &PieceTreeSlice) -> GraphemeCategory {
    let (chars, len) = {
        // read chars to a buf for easier handling
        let mut chars = ['\0'; 4];
        let mut n = 0;
        let mut iter = grapheme.chars();
        while let Some((_, _, ch)) = iter.next() {
            chars[n] = ch;
            n += 1;
        }
        (chars, n)
    };
    let chars = &chars[..len];

    if chars.iter().all(|ch| ch.is_alphanumeric() || *ch == '_') {
        return GraphemeCategory::Word;
    }

    if grapheme.is_eol() {
        return GraphemeCategory::EOL;
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
        let ch = Chars::new(&slice, 0, &DisplayOptions::default());
        assert_eq!("❤️ ", ch.display());
    }

    #[test]
    fn control_sequence_null() {
        let mut pt = PieceTree::new();
        pt.insert(0, b"\0");
        let slice = pt.slice(..);
        let ch = Chars::new(&slice, 0, &DisplayOptions::default());
        assert_eq!("^@", ch.display());
    }

    #[test]
    fn invalid_utf8() {
        let mut pt = PieceTree::new();
        pt.insert(0, b"\xFF");
        let slice = pt.slice(..);
        let ch = Chars::new(&slice, 0, &DisplayOptions::default());
        assert_eq!("\u{fffd}", ch.display());
    }

    #[test]
    fn tab() {
        let mut pt = PieceTree::new();
        pt.insert(0, "\t");
        let slice = pt.slice(..);
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
        let ch = Chars::new(&slice, 0, &DisplayOptions::default());
        assert_eq!(expected, ch.display());
    }

    #[test]
    fn non_breaking_space() {
        let mut pt = PieceTree::new();
        pt.insert(0, "\u{00A0}");
        let slice = pt.slice(..);
        let opts = DisplayOptions::default();
        let ch = Chars::new(&slice, 0, &opts);
        let expected = opts
            .replacements
            .get(&Replacement::NonBreakingSpace)
            .unwrap()
            .to_string();
        assert_eq!(expected, ch.display());
    }
}
