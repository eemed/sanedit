use crate::grapheme_category;
use crate::Chars;
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

pub fn next_paragraph(slice: &PieceTreeSlice, pos: u64) -> u64 {
    let mut lines = slice.lines_at(pos);

    // Skip all empty lines
    while let Some(line) = lines.next() {
        if !line.is_eol() {
            break;
        }
    }

    // Skip all content lines
    while let Some(line) = lines.next() {
        if line.is_eol() {
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
        if !line.is_eol() {
            break;
        }
    }

    // Skip all content lines
    while let Some(line) = lines.prev() {
        if line.is_eol() {
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
    let (npos, _) = pos_at_width(slice, pos, width, opts);
    (npos, width)
}

pub fn prev_line(slice: &PieceTreeSlice, cursor: &Cursor, opts: &DisplayOptions) -> (u64, usize) {
    let cpos = cursor.pos();
    let width = cursor
        .column()
        .unwrap_or_else(|| width_at_pos(slice, cpos, opts));
    let pos = prev_line_start(slice, cpos);
    let (npos, _) = pos_at_width(slice, pos, width, opts);
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

pub fn next_visual_line(
    slice: &PieceTreeSlice,
    cursor: &Cursor,
    opts: &DisplayOptions,
) -> (u64, usize) {
    let cpos = cursor.pos();
    let width = width_at_pos(slice, cpos, opts);
    // Atleast to next line, width wise or completely
    let mut pos = cpos;
    let mut col = width % opts.width;
    let mut total = width;
    let mut graphemes = slice.graphemes_at(cpos);
    let wrap_width = opts.wrap_char_width();
    let want_width = cursor.column().unwrap_or(width);

    // Goto next line, or next wrapped line
    while let Some(g) = graphemes.next() {
        let chars = Chars::new(&g, col, opts);
        let ch_width = chars.width();
        let ch_len = chars.len_in_buffer();
        let eol = chars.is_eol();

        if eol {
            // TODO only eol on next line
            // if col + ch_width > opts.width {
            //     return (pos, want_width)
            // }

            log::info!("EOL");
            total = 0;
            col = 0;
            pos += ch_len;
            break;
        }

        col += ch_width;

        if col > opts.width {
            total += wrap_width;
            col = wrap_width + chars.width();
            break;
        }

        total += ch_width;
        pos += ch_len;
    }

    let max_col = width % opts.width;
    // Scroll to same column as last
    while let Some(g) = graphemes.next() {
        let chars = Chars::new(&g, col, opts);
        let ch_width = chars.width();
        let ch_len = chars.len_in_buffer();
        let eol = chars.is_eol();

        if eol {
            log::info!("EOL2");
            return (pos, want_width);
        }

        col += ch_width;

        if col > max_col {
            col -= ch_width;
            graphemes.prev();
            break;
        }

        total += ch_width;
        pos += ch_len;
    }

    // Scroll to want column
    if let Some(want_width) = cursor.column() {
        log::info!("WNT COL: {want_width}, total: {total}");
        if want_width > total {
            while let Some(g) = graphemes.next() {
                let chars = Chars::new(&g, col, opts);
                let ch_width = chars.width();
                let ch_len = chars.len_in_buffer();
                let eol = chars.is_eol();

                if eol {
                    log::info!("RET1: total: {total}");
                    return (pos, total);
                }
                col += ch_width;

                if col > opts.width {
                    log::info!("RET2");
                    return (pos, total);
                }

                log::info!("total: +{ch_width}");
                total += ch_width;
                pos += ch_len;

                if total > want_width {
                    log::info!("RET3: total: {}", total - ch_width);
                    return (pos - ch_len, total - ch_width);
                }
            }
        }
    }

    (pos, want_width)
}

pub fn prev_visual_line(
    slice: &PieceTreeSlice,
    cursor: &Cursor,
    opts: &DisplayOptions,
) -> (u64, usize) {
    todo!()
    // let cpos = cursor.pos();
    // let width = cursor
    //     .column()
    //     .unwrap_or_else(|| width_at_pos(slice, cpos, opts));

    // // Try to find lower width on same line
    // // TODO we could find out if current line is even this length
    // let mut w = width;
    // while w >= opts.width {
    //     w -= opts.width;
    //     let (npos, found) = pos_at_width(slice, cpos, w, opts);
    //     if found && npos != cpos {
    //         return (npos, width);
    //     }
    // }

    // // Find highest width on previous line
    // let end = prev_line_end(slice, cpos);
    // let total_width = width_at_pos(slice, end, opts);
    // let width_off = width % opts.width;
    // let pline_target_width = (total_width / opts.width) * opts.width + width_off;
    // let (npos, _) = pos_at_width(slice, end, pline_target_width, opts);
    // (npos, pline_target_width)
}
