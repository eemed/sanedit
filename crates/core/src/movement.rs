use crate::grapheme_category;
use sanedit_buffer::utf8;
use sanedit_buffer::utf8::EndOfLine;
use sanedit_buffer::Bytes;
use sanedit_buffer::PieceTreeSlice;
use sanedit_buffer::Searcher;
use sanedit_buffer::SearcherRev;

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
    let mut start = bytes_start_of_line(&mut bytes);

    let mut graphemes = slice.graphemes_at(start);
    while let Some(grapheme) = graphemes.next() {
        let cat = grapheme_category(&grapheme);
        if cat != GraphemeCategory::Whitespace {
            start = grapheme.start();
            break;
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

        if let Some((ref prev, len)) = prev {
            if is_word_break_end(prev, &cat) {
                return pos - len;
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
    pos += graphemes.next().map(|g| g.len()).unwrap_or(0);

    while let Some(g) = graphemes.prev() {
        let prev = grapheme_category(&g);
        pos -= g.len();

        if let Some(cat) = cat {
            if is_word_break_end(&prev, &cat) {
                return pos;
            }
        }

        cat = Some(prev);
    }

    0
}

pub fn next_paragraph(slice: &PieceTreeSlice, pos: u64) -> u64 {
    let mut lines = slice.lines_at(pos);

    // Skip all empty lines
    while let Some(line) = lines.next() {
        if !EndOfLine::is_slice_eol(&line) {
            break;
        }
    }

    // Skip all content lines
    while let Some(line) = lines.next() {
        if EndOfLine::is_slice_eol(&line) {
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
        if !EndOfLine::is_slice_eol(&line) {
            break;
        }
    }

    // Skip all content lines
    while let Some(line) = lines.prev() {
        if EndOfLine::is_slice_eol(&line) {
            return line.start();
        }
    }

    0
}

fn find_next(slice: &PieceTreeSlice, pattern: &[u8]) -> Option<u64> {
    let searcher = Searcher::new(pattern);
    let mut iter = searcher.find_iter(slice);
    let mat = iter.next()?;
    Some(mat.start)
}

fn find_prev(slice: &PieceTreeSlice, pattern: &[u8]) -> Option<u64> {
    let searcher = SearcherRev::new(pattern);
    let mut iter = searcher.find_iter(slice);
    let mat = iter.next()?;
    Some(mat.start)
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
    log::info!("next line");
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
