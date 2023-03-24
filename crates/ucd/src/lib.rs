use std::cmp::Ordering;

mod general_category;
mod grapheme_break;
mod sentence_break;
mod word_break;

pub fn grapheme_break(ch: char) -> Option<&'static str> {
    search_table(
        ch,
        grapheme_break::GRAPHEME_CLUSTER_BREAK,
        grapheme_break::GRAPHEME_CLUSTER_BREAK_ENUM,
    )
}

fn search_table(
    ch: char,
    table: &'static [(u32, u32, u8)],
    enum_table: &'static [&'static str],
) -> Option<&'static str> {
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
    let kind = &enum_table[(*enum_pos) as usize];
    Some(kind)
}
