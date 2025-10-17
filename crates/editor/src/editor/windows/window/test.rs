use sanedit_core::IndentKind;
use sanedit_messages::redraw::Point;
use sanedit_utils::either::Either;

use super::*;

fn view_lines(win: &mut Window, buf: &Buffer) -> Vec<String> {
    fn to_str(row: &Vec<Cell>) -> String {
        let mut string = String::new();
        for cell in row {
            if let Some(ch) = cell.char() {
                match ch.display() {
                    Either::Left(s) => string.push_str(s),
                    Either::Right(ch) => string.push(ch),
                }
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

fn with_buf_size(content: &str, width: usize, height: usize) -> (Window, Buffer) {
    let mut buf = Buffer::new();
    let changes = Changes::multi_insert(&[0], content.as_bytes());
    let _ = buf.apply_changes(&changes);
    let mut win = Window::new(buf.id, width, height, WindowConfig::default());
    win.redraw_view(&buf);
    (win, buf)
}

fn assert_cursor_at(win: &Window, point: Point) {
    let pos = win.cursors.primary().pos();
    let cpoint = win.view.point_at_pos(pos).expect("Cursor not in view");
    assert_eq!(point, cpoint);
}

#[allow(dead_code)]
fn print_view(win: &Window) {
    println!("---");
    for line in win.view().cells() {
        print!("|");
        for cell in line {
            if let Some(ch) = cell.char() {
                match ch.display() {
                    Either::Left(s) => print!("{s}"),
                    Either::Right(ch) => print!("{ch}"),
                }
            } else {
                print!(" ")
            }
        }
        println!("|");
    }
    println!("---");
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

    win.cursors.cursors_mut().primary().goto(buf.len());
    let _ = win.indent_cursor_lines(&mut buf);
    let lines = view_lines(&mut win, &buf);
    assert_eq!(lines[0], "        hello");
}

#[test]
fn dedent() {
    let (mut win, mut buf) = with_buf("      ");
    buf.config.indent_kind = IndentKind::Space;
    buf.config.indent_amount = 4;

    win.cursors.cursors_mut().primary().goto(buf.len());
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

    win.cursors.cursors_mut().primary().goto(buf.len() - 5);
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

    win.cursors.cursors_mut().primary().select(13..buf.len());
    let _ = win.indent_cursor_lines(&mut buf);
    let lines = view_lines(&mut win, &buf);
    assert_eq!(lines[1], "    hello ");
    assert_eq!(lines[2], "    world");
}

#[test]
fn scroll_no_wrap() {
    let (mut win, buf) = with_buf_size("one\ntwo\nthree\nfour\nfive", 6, 3);
    win.scroll_down_n(&buf, 1);
    let lines = view_lines(&mut win, &buf);
    assert_eq!(lines[0], "two ");

    win.scroll_down_n(&buf, 1);
    let lines = view_lines(&mut win, &buf);
    assert_eq!(lines[0], "three ");

    win.scroll_up_n(&buf, 1);
    let lines = view_lines(&mut win, &buf);
    assert_eq!(lines[0], "two ");

    win.scroll_up_n(&buf, 1);
    let lines = view_lines(&mut win, &buf);
    assert_eq!(lines[0], "one ");
}

#[test]
fn scroll_wrap() {
    let (mut win, buf) = with_buf_size(
        "one long\ntwoo longer\nthree long\nfour long\nfivelong",
        9,
        3,
    );
    win.scroll_down_n(&buf, 1);
    let lines = view_lines(&mut win, &buf);
    assert_eq!(lines[0], "twoo long");

    win.scroll_down_n(&buf, 1);
    let lines = view_lines(&mut win, &buf);
    assert_eq!(lines[0], "↳er ");

    win.scroll_down_n(&buf, 1);
    let lines = view_lines(&mut win, &buf);
    assert_eq!(lines[0], "three lon");

    win.scroll_up_n(&buf, 1);
    let lines = view_lines(&mut win, &buf);
    assert_eq!(lines[0], "↳ longer ");

    win.scroll_up_n(&buf, 1);
    let lines = view_lines(&mut win, &buf);
    assert_eq!(lines[0], "twoo long");
}

#[test]
fn scroll_long_wrap() {
    let (mut win, buf) = with_buf_size("one longtwoo longerthree longfour longfivelong", 9, 3);
    win.scroll_down_n(&buf, 1);
    let lines = view_lines(&mut win, &buf);
    assert_eq!(lines[0], "↳woo long");

    win.scroll_down_n(&buf, 1);
    let lines = view_lines(&mut win, &buf);
    assert_eq!(lines[0], "↳erthree ");

    win.scroll_down_n(&buf, 1);
    let lines = view_lines(&mut win, &buf);
    assert_eq!(lines[0], "↳longfour");

    win.scroll_up_n(&buf, 1);
    let lines = view_lines(&mut win, &buf);
    assert_eq!(lines[0], "↳erthree ");

    win.scroll_up_n(&buf, 1);
    let lines = view_lines(&mut win, &buf);
    assert_eq!(lines[0], "↳woo long");

    win.scroll_up_n(&buf, 2);
    let lines = view_lines(&mut win, &buf);
    assert_eq!(lines[0], "one longt");
}

#[test]
fn cursor_zones_no_wrap() {
    let (mut win, buf) = with_buf_size("one\ntwo\nthree\nfour\nfive", 6, 3);
    win.goto_line(3, &buf);
    assert_cursor_at(&win, Point { x: 0, y: 2 });

    win.view_to_cursor_zone(&buf, Zone::Top);
    win.redraw_view(&buf);
    assert_cursor_at(&win, Point { x: 0, y: 0 });

    win.view_to_cursor_zone(&buf, Zone::Bottom);
    win.redraw_view(&buf);
    assert_cursor_at(&win, Point { x: 0, y: 2 });

    win.view_to_cursor_zone(&buf, Zone::Middle);
    win.redraw_view(&buf);
    assert_cursor_at(&win, Point { x: 0, y: 1 });

    win.cursors_to_lines_end(&buf);
    assert_cursor_at(&win, Point { x: 5, y: 1 });
    win.view_to_cursor_zone(&buf, Zone::Bottom);
    win.redraw_view(&buf);
    assert_cursor_at(&win, Point { x: 5, y: 2 });

    win.view_to_cursor_zone(&buf, Zone::Top);
    win.redraw_view(&buf);
    assert_cursor_at(&win, Point { x: 5, y: 0 });

    win.goto_line(2, &buf);
    win.cursor_to_view_zone(Zone::Top);
    assert_cursor_at(&win, Point { x: 0, y: 0 });

    win.cursor_to_view_zone(Zone::Middle);
    assert_cursor_at(&win, Point { x: 0, y: 1 });

    win.cursor_to_view_zone(Zone::Bottom);
    assert_cursor_at(&win, Point { x: 0, y: 2 });
}

#[test]
fn cursor_zones_wrap() {
    let (mut win, buf) = with_buf_size(
        "one long\ntwoo longer\nthree long\nfour long\nfivelong",
        9,
        3,
    );
    win.goto_line(2, &buf);
    win.cursors_to_lines_end(&buf);
    win.redraw_view(&buf);
    assert_cursor_at(&win, Point { x: 3, y: 2 });

    win.view_to_cursor_zone(&buf, Zone::Top);
    win.redraw_view(&buf);
    assert_cursor_at(&win, Point { x: 3, y: 0 });

    win.goto_line(3, &buf);
    win.cursors_to_lines_end(&buf);
    win.redraw_view(&buf);
    assert_cursor_at(&win, Point { x: 2, y: 2 });

    win.view_to_cursor_zone(&buf, Zone::Top);
    win.redraw_view(&buf);
    assert_cursor_at(&win, Point { x: 2, y: 0 });

    win.view_to_cursor_zone(&buf, Zone::Bottom);
    win.redraw_view(&buf);
    assert_cursor_at(&win, Point { x: 2, y: 2 });

    win.view_to_cursor_zone(&buf, Zone::Bottom);
    win.redraw_view(&buf);
    assert_cursor_at(&win, Point { x: 2, y: 2 });
}

#[test]
fn cursor_zones_long_wrap() {
    let (mut win, buf) = with_buf_size("one longtwoo longerthree longfour longfivelong", 9, 3);
    win.redraw_view(&buf);
    win.cursors.cursors_mut().primary().goto(9 * 2 - 1);
    assert_cursor_at(&win, Point { x: 1, y: 2 });

    win.view_to_cursor_zone(&buf, Zone::Top);
    win.redraw_view(&buf);
    assert_cursor_at(&win, Point { x: 1, y: 0 });

    win.view_to_cursor_zone(&buf, Zone::Middle);
    win.redraw_view(&buf);
    assert_cursor_at(&win, Point { x: 1, y: 1 });

    win.view_to_cursor_zone(&buf, Zone::Bottom);
    win.redraw_view(&buf);
    assert_cursor_at(&win, Point { x: 1, y: 2 });
}

#[test]
fn scroll_multi_line() {
    let (mut win, buf) = with_buf_size(
        "one long\ntwoo longer\nthree long\nfour long\nfivelong",
        9,
        3,
    );

    win.redraw_view(&buf);
    print_view(&win);
    win.scroll_down_n(&buf, 3);
    print_view(&win);
    win.scroll_up_n(&buf, 2);
    print_view(&win);
}

#[test]
fn pos_at_point_tabs() {
    let (mut win, buf) = with_buf_size("one long line is\n\t\twoo longer", 20, 3);

    win.redraw_view(&buf);
    assert_eq!(Some(0), win.view().pos_at_point(Point { x: 0, y: 0 }));
    assert_eq!(Some(17), win.view().pos_at_point(Point { x: 0, y: 1 }));
    assert_eq!(Some(17), win.view().pos_at_point(Point { x: 7, y: 1 }));
    assert_eq!(Some(18), win.view().pos_at_point(Point { x: 8, y: 1 }));
    assert_eq!(Some(18), win.view().pos_at_point(Point { x: 15, y: 1 }));
}

#[test]
fn pos_at_point_emoji() {
    let (mut win, buf) = with_buf_size("one\n🤍", 5, 3);

    win.redraw_view(&buf);
    assert_eq!(Some(0), win.view().pos_at_point(Point { x: 0, y: 0 }));
    assert_eq!(Some(4), win.view().pos_at_point(Point { x: 0, y: 1 }));
    assert_eq!(Some(4), win.view().pos_at_point(Point { x: 1, y: 1 }));
    assert_eq!(Some(8), win.view().pos_at_point(Point { x: 2, y: 1 }));
}
