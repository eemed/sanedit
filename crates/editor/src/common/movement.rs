use sanedit_buffer::utf8;
use sanedit_buffer::utf8::EndOfLine;
use sanedit_buffer::Bytes;
use sanedit_buffer::PieceTreeSlice;
use sanedit_buffer::Searcher;
use sanedit_buffer::SearcherRev;

use crate::common::char::grapheme_category;
use crate::editor::windows::Cursor;

use super::char::{is_word_break, is_word_break_end, DisplayOptions, GraphemeCategory};
use super::text::{pos_at_width, width_at_pos};

#[inline]
pub(crate) fn next_grapheme_boundary(slice: &PieceTreeSlice, pos: usize) -> usize {
    utf8::next_grapheme_boundary(slice, pos)
}

#[inline]
pub(crate) fn prev_grapheme_boundary(slice: &PieceTreeSlice, pos: usize) -> usize {
    utf8::prev_grapheme_boundary(slice, pos)
}

pub(crate) fn end_of_line(slice: &PieceTreeSlice, pos: usize) -> usize {
    let mut bytes = slice.bytes_at(pos);
    match utf8::next_eol(&mut bytes) {
        Some(m) => m.range.start,
        None => slice.len(),
    }
}

pub(crate) fn start_of_line(slice: &PieceTreeSlice, pos: usize) -> usize {
    let mut bytes = slice.bytes_at(pos);
    bytes_start_of_line(&mut bytes)
}

fn bytes_start_of_line(bytes: &mut Bytes) -> usize {
    match utf8::prev_eol(bytes) {
        Some(m) => m.range.end,
        None => 0,
    }
}

pub(crate) fn next_line_start(slice: &PieceTreeSlice, pos: usize) -> usize {
    let mut bytes = slice.bytes_at(pos);
    match utf8::next_eol(&mut bytes) {
        Some(m) => m.range.end,
        None => slice.len(),
    }
}

pub(crate) fn next_line_end(slice: &PieceTreeSlice, pos: usize) -> usize {
    let mut bytes = slice.bytes_at(pos);
    match utf8::next_eol(&mut bytes) {
        Some(m) => m.range.start,
        None => slice.len(),
    }
}

pub(crate) fn prev_line_start(slice: &PieceTreeSlice, pos: usize) -> usize {
    let mut bytes = slice.bytes_at(pos);
    utf8::prev_eol(&mut bytes);
    match utf8::prev_eol(&mut bytes) {
        Some(m) => m.range.end,
        None => 0,
    }
}

/// Find next word start, this will move even if we currently are on a word
/// start.
pub(crate) fn next_word_start(slice: &PieceTreeSlice, mut pos: usize) -> usize {
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
pub(crate) fn prev_word_start(slice: &PieceTreeSlice, mut pos: usize) -> usize {
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

pub(crate) fn next_word_end(slice: &PieceTreeSlice, mut pos: usize) -> usize {
    let mut prev: Option<(GraphemeCategory, usize)> = None;
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

pub(crate) fn prev_word_end(slice: &PieceTreeSlice, mut pos: usize) -> usize {
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

pub(crate) fn next_paragraph(slice: &PieceTreeSlice, pos: usize) -> usize {
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

pub(crate) fn prev_paragraph(slice: &PieceTreeSlice, pos: usize) -> usize {
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

fn find_next(slice: &PieceTreeSlice, pattern: &[u8]) -> Option<usize> {
    let searcher = Searcher::new(pattern);
    let mut iter = searcher.find_iter(slice);
    let mat = iter.next()?;
    Some(mat.start)
}

fn find_prev(slice: &PieceTreeSlice, pattern: &[u8]) -> Option<usize> {
    let searcher = SearcherRev::new(pattern);
    let mut iter = searcher.find_iter(slice);
    let mat = iter.next()?;
    Some(mat.start)
}

pub(crate) fn next_blank_line(slice: &PieceTreeSlice, pos: usize) -> usize {
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

pub(crate) fn prev_blank_line(slice: &PieceTreeSlice, pos: usize) -> usize {
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

pub(crate) fn next_line(
    slice: &PieceTreeSlice,
    cursor: &Cursor,
    opts: &DisplayOptions,
) -> (usize, usize) {
    let cpos = cursor.pos();
    let width = cursor
        .column()
        .unwrap_or_else(|| width_at_pos(slice, cpos, opts));
    let pos = next_line_start(slice, cpos);
    let npos = pos_at_width(slice, pos, width, opts);
    (npos, width)
}

pub(crate) fn prev_line(
    slice: &PieceTreeSlice,
    cursor: &Cursor,
    opts: &DisplayOptions,
) -> (usize, usize) {
    let cpos = cursor.pos();
    let width = cursor
        .column()
        .unwrap_or_else(|| width_at_pos(slice, cpos, opts));
    let pos = prev_line_start(slice, cpos);
    let npos = pos_at_width(slice, pos, width, opts);
    (npos, width)
}
