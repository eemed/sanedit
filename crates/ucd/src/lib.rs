mod enums;
mod general_category;
mod grapheme_break;
mod properties;
mod sentence_break;
mod word_break;

use std::cmp::Ordering;

pub use enums::GraphemeBreak;
pub use enums::Property;

pub fn grapheme_break(ch: char) -> GraphemeBreak {
    // Optimization for ascii
    // First entries in the GRAPHEME_CLUSTER_BREAK table
    // (0, 9, 1),     => Control
    // (10, 10, 4),   => LF
    // (11, 12, 1),   => Control
    // (13, 13, 0),   => CR
    // (14, 31, 1),   => Control
    // (127, 159, 1), => Control
    let num = ch as u32;
    if num <= 126 {
        if num >= 32 {
            GraphemeBreak::Any
        } else if num == 10 {
            GraphemeBreak::LF
        } else if num == 13 {
            GraphemeBreak::CR
        } else {
            GraphemeBreak::Control
        }
    } else {
        table_search(ch, grapheme_break::GRAPHEME_CLUSTER_BREAK)
            .map(|pos| {
                // SAFETY: index is from GRAPHEME_CLUSTER_BREAK_ENUM and GraphemeBreak is
                // just a rust enum version of it with repr(u8)
                unsafe { std::mem::transmute(pos) }
            })
            .unwrap_or(GraphemeBreak::Any)
    }
}

fn table_contains(ch: char, table: &'static [(u32, u32)]) -> bool {
    let ch = ch as u32;
    table
        .binary_search_by(|(start, end)| {
            if ch < *start {
                Ordering::Greater
            } else if *end < ch {
                Ordering::Less
            } else {
                Ordering::Equal
            }
        })
        .is_ok()
}

fn table_search(ch: char, table: &'static [(u32, u32, u8)]) -> Option<u8> {
    let ch = ch as u32;
    let pos = table
        .binary_search_by(|(start, end, _)| {
            if ch < *start {
                Ordering::Greater
            } else if *end < ch {
                Ordering::Less
            } else {
                Ordering::Equal
            }
        })
        .ok()?;
    let (_, _, enum_pos) = &table[pos];
    Some(*enum_pos)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn ascii() {
        let gb = grapheme_break('a');
        println!("GB {gb:?}");
    }
}
