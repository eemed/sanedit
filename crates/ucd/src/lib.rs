use std::cmp::Ordering;

mod general_category;
mod grapheme_break;
mod sentence_break;
mod word_break;
mod enums;

pub use enums::GraphemeBreak;

pub fn grapheme_break(ch: char) -> Option<GraphemeBreak> {
    let pos = search_table(
        ch,
        grapheme_break::GRAPHEME_CLUSTER_BREAK,
    )?;
    // SAFETY: index is from GRAPHEME_CLUSTER_BREAK_ENUM and GraphemeBreak is
    // just a rust enum version of it with repr(u8)
    Some( unsafe { std::mem::transmute(pos) })
}

fn search_table(
    ch: char,
    table: &'static [(u32, u32, u8)],
) -> Option<u8> {
    let ch = ch as u32;
    let pos = table
        .binary_search_by(|(start, end, _)| {
            if ch < *start {
                Ordering::Less
            } else if *end < ch {
                Ordering::Greater
            } else {
                Ordering::Equal
            }
        })
        .ok()?;
    let (_, _, enum_pos) = &table[pos];
    Some(*enum_pos)
}
