use sanedit_buffer::{utf8::EndOfLine, PieceTreeSlice};

use super::{
    char::{Char, DisplayOptions},
    movement::start_of_line,
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
        pos += ch.grapheme_len();
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
        pos += ch.grapheme_len();
    }

    pos
}

// pub(crate) fn on_word_end(slice: &PieceTreeSlice, mut pos: usize) -> bool {
//     let prev = next_grapheme(slice, pos);
//     pos += prev.as_ref().map_or(0, PieceTreeSlice::len);
//     let next = next_grapheme(slice, pos);

//     match (prev, next) {
//         (Some(p), Some(n)) => {
//             let p = grapheme_category(&p);
//             let n = grapheme_category(&n);
//             is_word_break_end(&p, &n)
//         }
//         _ => pos == slice.len(),
//     }
// }

// pub(crate) fn on_word_start(slice: &PieceTreeSlice, pos: usize) -> bool {
//     let prev = prev_grapheme(slice, pos).as_ref().map(grapheme_category);
//     let next = next_grapheme(slice, pos).as_ref().map(grapheme_category);

//     match (prev, next) {
//         (Some(p), Some(n)) => is_word_break(&p, &n),
//         _ => pos == 0,
//     }
// }

// /// Returns the range of the word that includes position pos
// pub(crate) fn word_at_pos(slice: &PieceTreeSlice, pos: usize) -> Option<Range<usize>> {
//     let cat = next_grapheme(slice, pos).as_ref().map(grapheme_category)?;
//     if !cat.is_word() {
//         return None;
//     }

//     let start = if on_word_start(slice, pos) {
//         pos
//     } else {
//         prev_word_start(slice, pos)
//     };

//     let end = {
//         let end = if on_word_end(slice, pos) {
//             pos
//         } else {
//             next_word_end(slice, pos)
//         };
//         next_grapheme_boundary(slice, end)
//     };

//     Some(start..end)
// }
