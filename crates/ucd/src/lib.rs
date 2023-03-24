use std::cmp::Ordering;

mod general_category;
mod grapheme_break;
mod sentence_break;
mod word_break;
mod enums;

pub use enums::GraphemeBreak;

pub fn grapheme_break(ch: char) -> GraphemeBreak {
    search_table(
        ch,
        grapheme_break::GRAPHEME_CLUSTER_BREAK,
    )
    .map(|pos| {
        // SAFETY: index is from GRAPHEME_CLUSTER_BREAK_ENUM and GraphemeBreak is
        // just a rust enum version of it with repr(u8)
        unsafe { std::mem::transmute(pos) }
    })
    .unwrap_or(GraphemeBreak::Any)
}

fn search_table(
    ch: char,
    table: &'static [(u32, u32, u8)],
) -> Option<u8> {
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
