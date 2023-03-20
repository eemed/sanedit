mod general_category;
mod grapheme_break;
mod word_break;
mod sentence_break;

pub fn is_grapheme_break(a: char, b: char) -> bool {
    todo!()
}

// pub fn is_word_break() -> bool {
// }

// pub fn is_sentence_break() -> bool {
// }

// pub fn general_category() {
// }
//
fn search(a: char, b: char, table: &[(char, char)]) {
    table.binary_search(&(a, b));
}
