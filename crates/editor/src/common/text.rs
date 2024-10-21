use sanedit_buffer::PieceTreeSlice;
use sanedit_core::{grapheme_category, movement::start_of_line, GraphemeCategory};

/// returns the line start if we are on EOL at a line with only whitespace chars
pub(crate) fn at_eol_on_whitespace_line(slice: &PieceTreeSlice, pos: u64) -> Option<u64> {
    let mut graphemes = slice.graphemes_at(pos);
    let eol_or_eof = graphemes.next().map(|g| g.is_eol()).unwrap_or(true);
    if !eol_or_eof {
        return None;
    }
    graphemes.prev();

    while let Some(g) = graphemes.prev() {
        if g.is_eol() {
            return Some(g.end());
        }

        let cat = grapheme_category(&g);
        if !matches!(cat, GraphemeCategory::Whitespace) {
            return None;
        }
    }

    None
}

/// Returns string to close pairs on line if pos is at eol
pub(crate) fn at_eol_close_pairs(slice: &PieceTreeSlice, pos: u64) -> String {
    let mut graphemes = slice.graphemes_at(pos);
    let eol_or_eof = graphemes.next().map(|g| g.is_eol()).unwrap_or(true);
    // If not on eol or eof, dont close
    if !eol_or_eof {
        return String::new();
    }

    graphemes.prev();

    // TODO tags
    // TODO ignore strings " ' `
    // configurable pairs, case esac in bash etc

    let start = start_of_line(slice, pos);
    let line = slice.slice(start..pos);
    let mut chars = line.chars();
    let mut result = vec![];

    while let Some((_, _, ch)) = chars.next() {
        match ch {
            '{' => {
                result.push('}');
            }
            '[' => {
                result.push(']');
            }
            '(' => {
                result.push(')');
            }
            ch => {
                if result.last() == Some(&ch) {
                    result.pop();
                }
            }
        }
    }
    result.into_iter().rev().fold(String::new(), |mut acc, ch| {
        acc.push(ch);
        acc
    })
}
