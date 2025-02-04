use std::io;

use crate::{
    grapheme_category, is_word_break, is_word_break_end,
    movement::{end_of_line, first_char_of_line},
    BufferRange, Chars, DisplayOptions, GraphemeCategory, Range,
};
use sanedit_buffer::{
    utf8::{self, prev_eol, EndOfLine},
    PieceTree, PieceTreeSlice,
};

use super::movement::{next_word_end, prev_word_start, start_of_line};

pub fn width_at_pos(slice: &PieceTreeSlice, pos: u64, opts: &DisplayOptions) -> usize {
    let target = pos;
    let mut pos = start_of_line(slice, pos);
    let mut col = 0;
    let mut graphemes = slice.graphemes_at(pos);

    while let Some(g) = graphemes.next() {
        let chars = Chars::new(&g, col, opts);
        if pos >= target {
            break;
        }

        col += chars.width();
        pos += chars.len_in_buffer();
    }

    col
}

/// returns the position at width + the width at the position
pub fn pos_at_width(slice: &PieceTreeSlice, pos: u64, width: usize, opts: &DisplayOptions) -> u64 {
    let mut pos = start_of_line(slice, pos);
    let mut col = 0;
    let mut graphemes = slice.graphemes_at(pos);

    while let Some(g) = graphemes.next() {
        let chars = Chars::new(&g, col, opts);
        let ch_width = chars.width();
        let ch_len = chars.len_in_buffer();
        let eol = chars.is_eol();

        if col + ch_width > width {
            break;
        }
        if eol {
            break;
        }
        col += ch_width;
        pos += ch_len;
    }

    pos
}

pub fn on_word_end(
    prev: (u64, GraphemeCategory),
    next: Option<(u64, GraphemeCategory)>,
    pos: u64,
    slice_len: u64,
) -> bool {
    match (prev, next) {
        ((_, p), Some((_, n))) => is_word_break_end(&p, &n),
        ((len, _), _) => pos + len == slice_len,
    }
}

pub fn on_word_start(
    prev: Option<(u64, GraphemeCategory)>,
    next: (u64, GraphemeCategory),
    pos: u64,
) -> bool {
    match (prev, next) {
        (Some((_, p)), (_, n)) => is_word_break(&p, &n),
        _ => pos == 0,
    }
}

/// Returns the range of the word that includes position pos
pub fn word_at_pos(slice: &PieceTreeSlice, pos: u64) -> Option<BufferRange> {
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
        end
    };

    Some(Range::new(start, end))
}

pub fn word_before_pos(slice: &PieceTreeSlice, pos: u64) -> Option<(BufferRange, String)> {
    let mut start = pos;
    let mut graphemes = slice.graphemes_at(pos);
    while let Some(g) = graphemes.prev() {
        use GraphemeCategory::*;
        match grapheme_category(&g) {
            Word => start = g.start(),
            _ => break,
        }
    }

    let word = slice.slice(start..pos);
    let word = String::from(&word);
    Some((Range::new(start, pos), word))
}

pub fn paragraph_at_pos(slice: &PieceTreeSlice, pos: u64) -> Option<BufferRange> {
    let mut lines = slice.lines_at(pos);
    let mut start = None;
    let mut end = None;

    // Skip all content lines up
    while let Some(line) = lines.prev() {
        if line.is_eol() {
            start = Some(line.end());
            lines.next();
            break;
        }
    }

    let start = start.unwrap_or(0);

    // Skip all content lines down
    while let Some(line) = lines.next() {
        if line.is_eol() {
            end = Some(line.start());
            break;
        }
    }

    let end = end.unwrap_or(slice.len());

    Some(Range::new(start, end))
}

pub fn strip_eol(slice: &mut PieceTreeSlice) {
    let mut bytes = slice.bytes_at(slice.len());
    if let Some(mat) = prev_eol(&mut bytes) {
        let end = slice.len() - mat.eol.len();
        *slice = slice.slice(..end);
    }
}

pub fn paste_separate_cursor_lines(text: &str) -> Vec<String> {
    let pt = PieceTree::from_reader(io::Cursor::new(text)).unwrap();
    let mut lines = pt.lines();
    let mut slices = vec![];

    while let Some(line) = lines.next() {
        if line.is_empty() {
            continue;
        }
        slices.push(line);
    }

    let mut result = vec![];
    let mut iter = slices.into_iter().peekable();
    while let Some(mut line) = iter.next() {
        if iter.peek().is_some() {
            strip_eol(&mut line);
        }

        let sline = String::from(&line);
        result.push(sline);
    }

    result
}

pub fn copy_cursors_to_lines(lines: Vec<String>, eol: EndOfLine) -> String {
    let mut result = String::new();
    for line in lines {
        if !result.is_empty() {
            result.push_str(eol.as_ref());
        }
        result.push_str(&line);
    }

    result
}

pub fn selection_first_chars_of_lines(slice: &PieceTreeSlice, sel: BufferRange) -> Vec<u64> {
    let sol = first_char_of_line(slice, sel.start);
    let mut starts = vec![sol];

    if sel.is_empty() {
        return starts;
    }

    let slice = slice.slice(sel);
    let mut lines = slice.lines();
    lines.next();

    while let Some(line) = lines.next() {
        if !line.is_empty() {
            let fchar = first_char_of_line(&line, 0);
            if fchar != line.start() {
                starts.push(fchar);
            }
        }
    }

    starts
}

pub fn selection_line_starts(slice: &PieceTreeSlice, sel: BufferRange) -> Vec<u64> {
    let mut starts = vec![];
    let start = sel.start;
    let sol = start_of_line(slice, start);

    if sel.is_empty() {
        return vec![sol];
    }

    let slice = slice.slice(sel);
    let mut lines = slice.lines();

    if sol != start {
        starts.push(sol);
        // Skip first line
        lines.next();
    }

    while let Some(line) = lines.next() {
        if !line.is_empty() {
            starts.push(line.start());
        }
    }

    starts
}

pub fn selection_line_ends(slice: &PieceTreeSlice, sel: BufferRange) -> Vec<u64> {
    let start = sel.start;
    let eol = end_of_line(slice, start);

    if sel.is_empty() {
        return vec![eol];
    }

    let mut ends = vec![eol];
    let mut bytes = slice.bytes_at(eol);
    loop {
        match utf8::next_eol(&mut bytes) {
            Some(eol) => {
                ends.push(eol.range.start);

                if bytes.pos() >= sel.end {
                    break;
                }
            }
            None => ends.push(slice.len()),
        }
    }

    ends
}

pub fn at_start_of_line(slice: &PieceTreeSlice, pos: u64) -> bool {
    let mut graphemes = slice.graphemes_at(pos);
    match graphemes.prev() {
        Some(g) => grapheme_category(&g) == GraphemeCategory::EOL,
        None => true,
    }
}
