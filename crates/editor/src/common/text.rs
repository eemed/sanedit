use std::{io, ops::Range};

use sanedit_buffer::{
    utf8::{prev_eol, EndOfLine},
    PieceTree, PieceTreeSlice,
};

use crate::common::movement::next_grapheme_boundary;

use super::{
    char::{
        grapheme_category, is_word_break, is_word_break_end, Char, DisplayOptions, GraphemeCategory,
    },
    movement::{next_word_end, prev_word_start, start_of_line},
};

pub(crate) fn width_at_pos(slice: &PieceTreeSlice, pos: usize, opts: &DisplayOptions) -> usize {
    let target = pos;
    let mut pos = start_of_line(slice, pos);
    let mut col = 0;
    let mut graphemes = slice.graphemes_at(pos);

    while let Some(g) = graphemes.next() {
        let ch = Char::new(&g, col, opts);
        if pos >= target {
            break;
        }

        col += ch.width();
        pos += ch.len_in_buffer();
    }

    col
}

/// returns the position at width + the width at the position
pub(crate) fn pos_at_width(
    slice: &PieceTreeSlice,
    pos: usize,
    width: usize,
    opts: &DisplayOptions,
) -> usize {
    let mut pos = start_of_line(slice, pos);
    let mut col = 0;
    let mut graphemes = slice.graphemes_at(pos);

    while let Some(g) = graphemes.next() {
        let ch = Char::new(&g, col, opts);
        if col + ch.width() > width {
            break;
        }
        if EndOfLine::is_slice_eol(&g) {
            break;
        }
        col += ch.width();
        pos += ch.len_in_buffer();
    }

    pos
}

pub(crate) fn on_word_end(
    prev: (usize, GraphemeCategory),
    next: Option<(usize, GraphemeCategory)>,
    pos: usize,
    slice_len: usize,
) -> bool {
    match (prev, next) {
        ((_, p), Some((_, n))) => is_word_break_end(&p, &n),
        ((len, _), _) => pos + len == slice_len,
    }
}

pub(crate) fn on_word_start(
    prev: Option<(usize, GraphemeCategory)>,
    next: (usize, GraphemeCategory),
    pos: usize,
) -> bool {
    match (prev, next) {
        (Some((_, p)), (_, n)) => is_word_break(&p, &n),
        _ => pos == 0,
    }
}

/// Returns the range of the word that includes position pos
pub(crate) fn word_at_pos(slice: &PieceTreeSlice, pos: usize) -> Option<Range<usize>> {
    let make_pair = |slice: PieceTreeSlice| {
        let len = slice.len();
        let cat = grapheme_category(&slice);
        (len, cat)
    };
    let mut graphemes = slice.graphemes_at(pos);

    let before = if graphemes.prev().is_none() {
        None
    } else {
        graphemes.next().map(make_pair)
    };
    let current = graphemes.next().map(make_pair)?;
    let after = graphemes.next().map(make_pair);

    if !current.1.is_word() {
        return None;
    }

    let start = {
        let mut start = pos;
        if !on_word_start(before, current, pos) {
            start = prev_word_start(slice, pos);
        }
        start
    };

    let end = {
        let mut end = pos;
        if !on_word_end(current, after, pos, slice.len()) {
            end = next_word_end(slice, pos);
        }
        next_grapheme_boundary(slice, end)
    };

    Some(start..end)
}

pub(crate) fn strip_eol(slice: &mut PieceTreeSlice) {
    let mut bytes = slice.bytes_at(slice.len());
    if let Some(mat) = prev_eol(&mut bytes) {
        let end = slice.len() - mat.eol.len();
        *slice = slice.slice(..end);
    }
}

pub(crate) fn as_lines(text: &str) -> Vec<String> {
    let pt = PieceTree::from_reader(io::Cursor::new(text)).unwrap();
    let mut lines = pt.lines();
    let mut result = vec![];

    while let Some(mut line) = lines.next() {
        if line.is_empty() {
            continue;
        }
        strip_eol(&mut line);
        let sline = String::from(&line);
        result.push(sline);
    }

    result
}

pub(crate) fn to_line(lines: Vec<String>, eol: EndOfLine) -> String {
    let mut result = String::new();
    for line in lines {
        if !result.is_empty() {
            result.push_str(eol.as_ref());
        }
        result.push_str(&line);
    }

    result
}
