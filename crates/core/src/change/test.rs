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
fn cursor_insert_after_() {
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
