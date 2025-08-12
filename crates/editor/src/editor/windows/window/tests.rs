use sanedit_core::IndentKind;

use super::*;

fn view_lines(win: &mut Window, buf: &Buffer) -> Vec<String> {
    fn to_str(row: &Vec<Cell>) -> String {
        let mut string = String::new();
        for cell in row {
            if let Some(ch) = cell.char() {
                string.push_str(&ch.display());
            }
        }
        string
    }
    win.redraw_view(buf);
    win.view().cells().iter().map(to_str).collect()
}

fn with_buf(content: &str) -> (Window, Buffer) {
    let mut buf = Buffer::new();
    let changes = Changes::multi_insert(&[0], content.as_bytes());
    let _ = buf.apply_changes(&changes);
    let mut win = Window::new(buf.id, 50, 10, WindowConfig::default());
    win.redraw_view(&buf);
    (win, buf)
}

#[test]
fn indent() {
    let (mut win, mut buf) = with_buf("hello");
    buf.config.tabstop = 8;
    buf.config.indent_kind = IndentKind::Space;
    buf.config.indent_amount = 4;

    let _ = win.indent_cursor_lines(&mut buf);
    let lines = view_lines(&mut win, &buf);
    assert_eq!(lines[0], "    hello");

    win.cursors.primary_mut().goto(buf.len());
    let _ = win.indent_cursor_lines(&mut buf);
    let lines = view_lines(&mut win, &buf);
    assert_eq!(lines[0], "        hello");
}

#[test]
fn dedent() {
    let (mut win, mut buf) = with_buf("      ");
    buf.config.indent_kind = IndentKind::Space;
    buf.config.indent_amount = 4;

    win.cursors.primary_mut().goto(buf.len());
    let _ = win.dedent_cursor_lines(&mut buf);
    let lines = view_lines(&mut win, &buf);
    assert_eq!(lines[0], "    ");

    let _ = win.dedent_cursor_lines(&mut buf);
    let lines = view_lines(&mut win, &buf);
    assert_eq!(lines[0], "");
}

#[test]
fn dedent_missing_spaces() {
    let (mut win, mut buf) = with_buf("hello\n  bar");
    buf.config.indent_kind = IndentKind::Space;
    buf.config.indent_amount = 4;

    win.cursors.primary_mut().goto(buf.len() - 5);
    let _ = win.dedent_cursor_lines(&mut buf);
    let lines = view_lines(&mut win, &buf);
    assert_eq!(lines[0], "hello ");
    assert_eq!(lines[1], "bar");
}

#[test]
fn indent_multiline() {
    let (mut win, mut buf) = with_buf("uselessline\nhello\nworld");
    buf.config.tabstop = 8;
    buf.config.indent_kind = IndentKind::Space;
    buf.config.indent_amount = 4;

    win.cursors.primary_mut().select(13..buf.len());
    let _ = win.indent_cursor_lines(&mut buf);
    let lines = view_lines(&mut win, &buf);
    assert_eq!(lines[1], "    hello ");
    assert_eq!(lines[2], "    world");
}
