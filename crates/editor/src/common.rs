pub(crate) mod cursors;
pub(crate) mod window;

pub(crate) fn is_yes(input: &str) -> bool {
    input.eq_ignore_ascii_case("y")
        || input.eq_ignore_ascii_case("ye")
        || input.eq_ignore_ascii_case("yes")
}
