use crate::grapheme_category;
use sanedit_buffer::utf8;
use sanedit_buffer::utf8::EndOfLine;
use sanedit_buffer::Bytes;
use sanedit_buffer::PieceTreeSlice;

use crate::Cursor;

use super::text::{pos_at_width, width_at_pos};
use crate::{is_word_break, is_word_break_end, DisplayOptions, GraphemeCategory};

#[inline]
pub fn next_grapheme_boundary(slice: &PieceTreeSlice, pos: u64) -> u64 {
    utf8::next_grapheme_boundary(slice, pos)
}

#[inline]
pub fn prev_grapheme_boundary(slice: &PieceTreeSlice, pos: u64) -> u64 {
    utf8::prev_grapheme_boundary(slice, pos)
}

pub fn end_of_line(slice: &PieceTreeSlice, pos: u64) -> u64 {
    let mut bytes = slice.bytes_at(pos);
    match utf8::next_eol(&mut bytes) {
        Some(m) => m.range.start,
        None => slice.len(),
    }
}

pub fn start_of_line(slice: &PieceTreeSlice, pos: u64) -> u64 {
    let mut bytes = slice.bytes_at(pos);
    bytes_start_of_line(&mut bytes)
}

pub fn first_char_of_line(slice: &PieceTreeSlice, pos: u64) -> u64 {
    let mut bytes = slice.bytes_at(pos);
    let start = bytes_start_of_line(&mut bytes);

    let mut graphemes = slice.graphemes_at(start);
    while let Some(grapheme) = graphemes.next() {
        let cat = grapheme_category(&grapheme);
        if cat != GraphemeCategory::Whitespace {
           return grapheme.start() - slice.start();
        }
    }

    start
}

fn bytes_start_of_line(bytes: &mut Bytes) -> u64 {
    match utf8::prev_eol(bytes) {
        Some(m) => m.range.end,
        None => 0,
    }
}

pub fn next_line_start(slice: &PieceTreeSlice, pos: u64) -> u64 {
    let mut bytes = slice.bytes_at(pos);
    match utf8::next_eol(&mut bytes) {
        Some(m) => m.range.end,
        None => slice.len(),
    }
}

pub fn next_line_end(slice: &PieceTreeSlice, pos: u64) -> u64 {
    let mut bytes = slice.bytes_at(pos);
    match utf8::next_eol(&mut bytes) {
        Some(m) => m.range.start,
        None => slice.len(),
    }
}

pub fn prev_line_start(slice: &PieceTreeSlice, pos: u64) -> u64 {
    let mut bytes = slice.bytes_at(pos);
    utf8::prev_eol(&mut bytes);
    match utf8::prev_eol(&mut bytes) {
        Some(m) => m.range.end,
        None => 0,
    }
}

pub fn prev_line_end(slice: &PieceTreeSlice, pos: u64) -> u64 {
    let mut bytes = slice.bytes_at(pos);
    match utf8::prev_eol(&mut bytes) {
        Some(m) => m.range.start,
        None => 0,
    }
}

/// Find next word start, this will move even if we currently are on a word
/// start.
pub fn next_word_start(slice: &PieceTreeSlice, mut pos: u64) -> u64 {
    let mut prev: Option<GraphemeCategory> = None;
    let mut graphemes = slice.graphemes_at(pos);

    while let Some(g) = graphemes.next() {
        let cat = grapheme_category(&g);

        if let Some(ref prev) = prev {
            if is_word_break(prev, &cat) {
                return pos;
            }
        }

        pos += g.len();
        prev = Some(cat);
    }

    slice.len()
}

/// Find previous word start, this will move even if we currently are on a word
/// start.
pub fn prev_word_start(slice: &PieceTreeSlice, mut pos: u64) -> u64 {
    let mut cat: Option<GraphemeCategory> = None;
    let mut graphemes = slice.graphemes_at(pos);

    while let Some(g) = graphemes.prev() {
        let prev = grapheme_category(&g);

        if let Some(cat) = cat {
            if is_word_break(&prev, &cat) {
                return pos;
            }
        }

        pos -= g.len();
        cat = Some(prev);
    }

    0
}

pub fn next_word_end(slice: &PieceTreeSlice, mut pos: u64) -> u64 {
    let mut prev: Option<(GraphemeCategory, u64)> = None;
    let mut graphemes = slice.graphemes_at(pos);
    pos += graphemes.next().map(|g| g.len()).unwrap_or(0);

    while let Some(g) = graphemes.next() {
        let cat = grapheme_category(&g);

        if let Some((ref prev, _len)) = prev {
            if is_word_break_end(prev, &cat) {
                return pos;
            }
        }

        pos += g.len();
        prev = Some((cat, g.len()));
    }

    slice.len()
}

/// Find next word end and move to the next character
pub fn next_word_end_next(slice: &PieceTreeSlice, mut pos: u64) -> u64 {
    let mut prev: Option<(GraphemeCategory, u64)> = None;
    let mut graphemes = slice.graphemes_at(pos);
    pos += graphemes.next().map(|g| g.len()).unwrap_or(0);

    while let Some(g) = graphemes.next() {
        let cat = grapheme_category(&g);

        if let Some((ref prev, _len)) = prev {
            if is_word_break_end(prev, &cat) {
                return pos;
            }
        }

        pos += g.len();
        prev = Some((cat, g.len()));
    }

    slice.len()
}

pub fn prev_word_end(slice: &PieceTreeSlice, mut pos: u64) -> u64 {
    let mut cat: Option<GraphemeCategory> = None;
    let mut graphemes = slice.graphemes_at(pos);

    while let Some(g) = graphemes.prev() {
        let prev = grapheme_category(&g);
        pos -= g.len();

        if let Some(cat) = cat {
            if is_word_break_end(&prev, &cat) {
                return pos + g.len();
            }
        }

        cat = Some(prev);
    }

    0
}

/// Check whether slice is only whitespace and eols
pub fn is_empty_or_whitespace(slice: &PieceTreeSlice) -> bool {
    let mut graphemes = slice.graphemes();
    while let Some(g) = graphemes.next() {
        let cat = grapheme_category(&g);
        if !matches!(cat, GraphemeCategory::EOL | GraphemeCategory::Whitespace) {
            return false;
        }
    }

    true
}

pub fn next_paragraph(slice: &PieceTreeSlice, pos: u64) -> u64 {
    let mut lines = slice.lines_at(pos);

    // Skip all empty lines
    while let Some(line) = lines.next() {
        if !is_empty_or_whitespace(&line) {
            break;
        }
    }

    // Skip all content lines
    while let Some(line) = lines.next() {
        if is_empty_or_whitespace(&line) {
            return line.start();
        }
    }

    slice.len()
}

pub fn prev_paragraph(slice: &PieceTreeSlice, pos: u64) -> u64 {
    let mut lines = slice.lines_at(pos);
    lines.next();

    // Skip all empty lines
    while let Some(line) = lines.prev() {
        if !is_empty_or_whitespace(&line) {
            break;
        }
    }

    // Skip all content lines
    while let Some(line) = lines.prev() {
        if is_empty_or_whitespace(&line) {
            return line.start();
        }
    }

    0
}

pub fn next_blank_line(slice: &PieceTreeSlice, pos: u64) -> u64 {
    let mut bytes = slice.bytes_at(pos);
    utf8::next_eol(&mut bytes);

    let mut target = bytes.pos();
    while let Some(mat) = utf8::next_eol(&mut bytes) {
        if mat.range.start == target {
            return target;
        }

        target = mat.range.end;
    }

    slice.len()
}

pub fn prev_blank_line(slice: &PieceTreeSlice, pos: u64) -> u64 {
    let mut bytes = slice.bytes_at(pos);
    utf8::prev_eol(&mut bytes);

    let mut target = bytes.pos();
    while let Some(mat) = utf8::prev_eol(&mut bytes) {
        if mat.range.end == target {
            return target;
        }

        target = mat.range.start;
    }

    0
}

pub fn next_line(slice: &PieceTreeSlice, cursor: &Cursor, opts: &DisplayOptions) -> (u64, usize) {
    let cpos = cursor.pos();
    let width = cursor
        .column()
        .unwrap_or_else(|| width_at_pos(slice, cpos, opts));
    let pos = next_line_start(slice, cpos);
    let npos = pos_at_width(slice, pos, width, opts);
    (npos, width)
}

pub fn prev_line(slice: &PieceTreeSlice, cursor: &Cursor, opts: &DisplayOptions) -> (u64, usize) {
    let cpos = cursor.pos();
    let width = cursor
        .column()
        .unwrap_or_else(|| width_at_pos(slice, cpos, opts));
    let pos = prev_line_start(slice, cpos);
    let npos = pos_at_width(slice, pos, width, opts);
    (npos, width)
}

pub fn find_next_char(
    slice: &PieceTreeSlice,
    pos: u64,
    target: char,
    stop_at_eol: bool,
) -> Option<u64> {
    let mut chars = slice.chars_at(pos);

    while let Some((start, _, ch)) = chars.next() {
        if ch == target {
            return Some(start);
        }

        if stop_at_eol && EndOfLine::is_eol_char(ch) {
            return None;
        }
    }

    None
}

pub fn find_prev_char(
    slice: &PieceTreeSlice,
    pos: u64,
    target: char,
    stop_at_eol: bool,
) -> Option<u64> {
    let mut chars = slice.chars_at(pos);

    while let Some((start, _, ch)) = chars.prev() {
        if ch == target {
            return Some(start);
        }

        if stop_at_eol && EndOfLine::is_eol_char(ch) {
            return None;
        }
    }

    None
}

pub fn next_grapheme_on_line(slice: &PieceTreeSlice, pos: u64) -> u64 {
    let mut chars = slice.chars_at(pos);

    while let Some((start, end, ch)) = chars.next() {
        if EndOfLine::is_eol_char(ch) {
            return start;
        }

        return end;
    }

    pos
}

pub fn prev_grapheme_on_line(slice: &PieceTreeSlice, pos: u64) -> u64 {
    let mut chars = slice.chars_at(pos);

    while let Some((start, end, ch)) = chars.prev() {
        if EndOfLine::is_eol_char(ch) {
            return end;
        }

        return start;
    }

    pos
}

pub fn find_prev_whitespace(slice: &PieceTreeSlice, pos: u64) -> Option<u64> {
    let mut graphemes = slice.graphemes_at(pos);

    while let Some(g) = graphemes.prev() {
        let cat = grapheme_category(&g);
        if cat == GraphemeCategory::Whitespace {
            return Some(g.start());
        }
    }

    None
}
