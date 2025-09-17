use super::*;

#[test]
fn cursor_insert_after_border() {
    // Cursor:  |-----------|
    // Change:              |+++++++++|
    // Result:  |-----------|

    let mut cursors = vec![Cursor::new_select(0..10)];
    let changes = Changes::new(&[Change::insert(10, b"hello")]);
    changes.move_cursors(&mut cursors, false);
    let cursor = &cursors[0];
    assert_eq!(0, cursor.start());
    assert_eq!(10, cursor.end());
}

#[test]
fn cursor_insert_after() {
    // Cursor:  |-----------|
    // Change:                   |+++++++++|
    // Result:  |-----------|

    let mut cursors = vec![Cursor::new_select(0..10)];
    let changes = Changes::new(&[Change::insert(12, b"hello")]);
    changes.move_cursors(&mut cursors, false);
    let cursor = &cursors[0];
    assert_eq!(0, cursor.start());
    assert_eq!(10, cursor.end());
}

#[test]
fn cursor_insert_before_border() {
    // Cursor:           |-----------|
    // Change: |+++++++++|
    // Result:           |-----------|

    let mut cursors = vec![Cursor::new_select(10..20)];
    let changes = Changes::new(&[Change::insert(9, b"hello")]);
    changes.move_cursors(&mut cursors, false);
    let cursor = &cursors[0];
    assert_eq!(15, cursor.start());
    assert_eq!(25, cursor.end());
}

#[test]
fn cursor_insert_before() {
    // Cursor:              |-----------|
    // Change: |+++++++++|
    // Result:              |-----------|

    let mut cursors = vec![Cursor::new_select(10..20)];
    let changes = Changes::new(&[Change::insert(5, b"hello")]);
    changes.move_cursors(&mut cursors, false);
    let cursor = &cursors[0];
    assert_eq!(15, cursor.start());
    assert_eq!(25, cursor.end());
}

#[test]
fn cursor_insert_middle() {
    // Cursor:  |-----------|
    // Change:        |+++++++++|
    // Result:  |-----|+++++++++|---|

    let mut cursors = vec![Cursor::new_select(0..10)];
    let changes = Changes::new(&[Change::insert(7, b"hello")]);
    changes.move_cursors(&mut cursors, false);
    let cursor = &cursors[0];
    assert_eq!(0, cursor.start());
    assert_eq!(15, cursor.end());
}

#[test]
fn cursor_insert_before_overlap() {
    // Cursor:      |-----------|
    // Change: |+++++++++|
    // Result:           |-----------|

    let mut cursors = vec![Cursor::new_select(10..20)];
    let changes = Changes::new(&[Change::insert(8, b"hello")]);
    changes.move_cursors(&mut cursors, false);
    let cursor = &cursors[0];
    assert_eq!(15, cursor.start());
    assert_eq!(25, cursor.end());
}

#[test]
fn cursor_insert_contains() {
    // Cursor: |---------------|
    // Change:    |++++++|
    // Result: |--|++++++|-----------|

    let mut cursors = vec![Cursor::new_select(0..10)];
    let changes = Changes::new(&[Change::insert(2, b"hello")]);
    changes.move_cursors(&mut cursors, false);
    let cursor = &cursors[0];
    assert_eq!(0, cursor.start());
    assert_eq!(15, cursor.end());
}

#[test]
fn cursor_insert_middle_of_change() {
    // Cursor:  |
    // Change:  |++++o++++|
    // Result:       |

    let mut cursors = vec![Cursor::new(0)];
    let mut change = Change::insert(0, b"helloworld");
    change.cursor_offset = Some(5);
    let changes = Changes::new(&[change]);
    changes.move_cursors(&mut cursors, false);
    let cursor = &cursors[0];
    assert_eq!(5, cursor.pos());
}

#[test]
fn cursor_select_replacement() {
    // Cursor1:  |xxxx|
    // Cursor2:            |yyyy|
    // Swap selections
    // Cursor1:  |yyyy|
    // Cursor2:            |xxxx|

    let mut cursors = vec![Cursor::new_select(1..5), Cursor::new_select(10..16)];
    let changes = Changes::new(&[Change::replace(1..5, b"abcd"), Change::replace(10..16, b"efghd")]);
    changes.move_cursors(&mut cursors, true);
    let cursor = &cursors[0];
    assert_eq!(1, cursor.start());
    assert_eq!(5, cursor.end());

    let cursor = &cursors[1];
    assert_eq!(10, cursor.start());
    assert_eq!(15, cursor.end());
}

#[test]
fn cursor_multi_insert() {
    let mut cursors = vec![Cursor::new(1), Cursor::new(3), Cursor::new(5)];
    let changes = Changes::multi_insert(&[1, 3, 5], b"foo");
    changes.move_cursors(&mut cursors, true);
    assert_eq!(4, cursors[0].start());
    assert_eq!(9, cursors[1].start());
    assert_eq!(14, cursors[2].start());
}

#[test]
fn cursor_multi_remove() {
    let mut cursors = vec![Cursor::new(1), Cursor::new(3), Cursor::new(5)];
    let changes = Changes::multi_remove(&[BufferRange::from(0..1), BufferRange::from(2..3), BufferRange::from(4..5)]);
    changes.move_cursors(&mut cursors, true);
    assert_eq!(0, cursors[0].start());
    assert_eq!(1, cursors[1].start());
    assert_eq!(2, cursors[2].start());
}

#[test]
fn cursor_select_remove() {
    let mut cursors = vec![Cursor::new_select(1..5)];
    let changes = Changes::new(&[Change::replace(1..5, b"p")]);
    changes.move_cursors(&mut cursors, false);
    assert_eq!(2, cursors[0].start());
    assert_eq!(2, cursors[0].end());
}
