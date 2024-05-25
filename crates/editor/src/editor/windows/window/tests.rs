use crate::common::indent::{Indent, IndentKind};

use super::*;

fn view_lines(win: &mut Window, buf: &Buffer) -> Vec<String> {
    fn to_str(row: &Vec<Cell>) -> String {
        let mut string = String::new();
        for cell in row {
            if let Some(ch) = cell.char() {
                string.push_str(ch.display());
            }
        }
        string
    }
    win.redraw_view(buf);
    win.view().cells().iter().map(to_str).collect()
}

fn with_buf(content: &str) -> (Window, Buffer) {
    let mut buf = Buffer::new();
    buf.insert(0, content);
    let mut win = Window::new(buf.id, 50, 10);
    win.redraw_view(&buf);
    (win, buf)
}

#[test]
fn insert_tab() {
    let (mut win, mut buf) = with_buf("");
    buf.options.indent = Indent {
        n: 4,
        kind: IndentKind::Space,
    };

    win.insert_tab(&mut buf);
    let lines = view_lines(&mut win, &buf);
    assert_eq!(lines[0], buf.options.indent.to_string());

    win.insert_at_cursors(&mut buf, "  ");
    win.insert_tab(&mut buf);
    let lines = view_lines(&mut win, &buf);
    assert_eq!(lines[0], buf.options.indent.get(2));
}

#[test]
fn insert_tab_text() {
    let (mut win, mut buf) = with_buf("hello");
    buf.options.tabstop = 8;
    buf.options.indent = Indent {
        n: 4,
        kind: IndentKind::Space,
    };

    win.cursors.primary_mut().goto(buf.len());
    win.insert_tab(&mut buf);
    let lines = view_lines(&mut win, &buf);
    assert_eq!(lines[0], "hello→  ".to_string());
}

#[test]
fn backtab() {
    let (mut win, mut buf) = with_buf("      ");
    buf.options.indent = Indent {
        n: 4,
        kind: IndentKind::Space,
    };

    win.cursors.primary_mut().goto(buf.len());
    win.backtab(&mut buf);
    let lines = view_lines(&mut win, &buf);
    assert_eq!(lines[0], buf.options.indent.to_string());

    win.backtab(&mut buf);
    let lines = view_lines(&mut win, &buf);
    assert_eq!(lines[0], "".to_string());
}

#[test]
fn remove_grapheme_before() {
    let (mut win, mut buf) = with_buf("      a\na");
    buf.options.indent = Indent {
        n: 4,
        kind: IndentKind::Space,
    };

    win.cursors.primary_mut().goto(buf.len());
    win.remove_grapheme_before_cursors(&mut buf);
    let lines = view_lines(&mut win, &buf);
    assert_eq!(lines[0], "      a ");

    win.remove_grapheme_before_cursors(&mut buf);
    let lines = view_lines(&mut win, &buf);
    assert_eq!(lines[0], "      a");

    win.remove_grapheme_before_cursors(&mut buf);
    let lines = view_lines(&mut win, &buf);
    assert_eq!(lines[0], "      ");

    win.remove_grapheme_before_cursors(&mut buf);
    let lines = view_lines(&mut win, &buf);
    assert_eq!(lines[0], "    ");

    win.remove_grapheme_before_cursors(&mut buf);
    let lines = view_lines(&mut win, &buf);
    assert_eq!(lines[0], "");
}
