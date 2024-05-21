use super::*;

fn to_str(row: &Vec<Cell>) -> String {
    let mut string = String::new();
    for cell in row {
        if let Some(ch) = cell.char() {
            string.push_str(ch.display());
        }
    }
    string
}

fn view_lines(win: &Window) -> Vec<String> {
    win.view().cells().iter().map(to_str).collect()
}

fn wrapped_line_view() -> (Window, Buffer) {
    let mut buf = Buffer::new();
    buf.insert(
            0,
            "this is a long line that will not fit\nthis is another long line that will not fit into the view\nthis is the third line that is longer than the view",
        );
    let mut win = Window::new(buf.id, 10, 3);
    win.redraw_view(&buf);
    (win, buf)
}

fn ten_line() -> (Window, Buffer) {
    let mut buf = Buffer::new();
    buf.insert(
        0,
        "one\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten",
    );
    let mut win = Window::new(buf.id, 80, 3);
    win.redraw_view(&buf);
    (win, buf)
}

#[test]
fn scroll_up() {
    let (mut win, buf) = ten_line();
    win.view.set_offset(14);
    win.view.redraw(&buf);
    assert_eq!(vec!["four\n", "five\n", "six\n"], view_lines(&win));
    win.scroll_up_n(&buf, 2);
    assert_eq!(vec!["two\n", "three\n", "four\n"], view_lines(&win));
}

#[test]
fn scroll_up_wrapped() {
    let (mut win, buf) = wrapped_line_view();
    win.view.set_offset(52);
    win.view.redraw(&buf);
    assert_eq!(
        vec!["r long lin", "e that wil", "l not fit "],
        view_lines(&win)
    );

    win.scroll_up_n(&buf, 1);
    assert_eq!(
        vec!["this is an", "other long", " line that"],
        view_lines(&win)
    );

    win.scroll_up_n(&buf, 1);
    assert_eq!(
        vec!["a long lin", "e that wil", "l not fit\n"],
        view_lines(&win)
    );
}

#[test]
fn scroll_down() {
    let (mut win, buf) = ten_line();
    win.scroll_down_n(&buf, 2);
    assert_eq!(vec!["three\n", "four\n", "five\n"], view_lines(&win));
}

#[test]
fn scroll_down_wrapped() {
    let (mut win, buf) = wrapped_line_view();
    win.scroll_down_n(&buf, 2);
    assert_eq!(
        vec!["that will ", "not fit\n", "this is an"],
        view_lines(&win)
    );
}

#[test]
fn view_to_after() {
    // let (mut win, buf) = wrapped_line_view();
    // assert_eq!(vec!["", "", ""], view_lines(&win));
}

#[test]
fn view_to_after_small() {}

#[test]
fn view_to_before() {}

#[test]
fn view_to_before_small() {}
