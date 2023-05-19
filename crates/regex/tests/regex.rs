use sanedit_regex::{Regex, StringCursor};

#[test]
fn question_no_match() {
    let regex = Regex::new("car?").unwrap();
    let mut text: StringCursor = "cab".into();
    let matched = regex.find(&mut text);
    assert!(matched.is_some());
    let matched = matched.unwrap();
    assert_eq!(0..2, matched.range());
}

#[test]
fn question_match() {
    let regex = Regex::new("car?").unwrap();
    let mut text: StringCursor = "carb".into();
    let matched = regex.find(&mut text);
    assert!(matched.is_some());
    let matched = matched.unwrap();
    assert_eq!(0..3, matched.range());
}

#[test]
fn any_byte_greedy() {
    let mut text: StringCursor = "foo\nbar\nbaz\n".into();

    let regex = Regex::new("bar.*").unwrap();
    let matched = regex.find(&mut text);
    assert!(matched.is_some());
    let matched = matched.unwrap();
    assert_eq!(4..12, matched.range());
}

#[test]
fn any_byte_lazy() {
    let mut text: StringCursor = "foo\nbar\nbaz\n".into();

    let regex = Regex::new("bar.*?").unwrap();
    let matched = regex.find(&mut text);
    assert!(matched.is_some());
    let matched = matched.unwrap();
    assert_eq!(4..7, matched.range());
}
