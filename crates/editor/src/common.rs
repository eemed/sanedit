pub(crate) mod char;
pub(crate) mod cursors;
pub(crate) mod dirs;
pub(crate) mod file;
pub(crate) mod indent;
pub(crate) mod matcher;
pub(crate) mod movement;
pub(crate) mod pairs;
pub(crate) mod range;
pub(crate) mod search;
pub(crate) mod text;
pub(crate) mod text_objects;
pub(crate) mod window;

pub(crate) fn is_yes(input: &str) -> bool {
    input.eq_ignore_ascii_case("y")
        || input.eq_ignore_ascii_case("ye")
        || input.eq_ignore_ascii_case("yes")
}
