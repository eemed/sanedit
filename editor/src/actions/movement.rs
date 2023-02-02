use std::cmp;

use sanedit_messages::redraw::Point;

use crate::{common, editor::Editor, server::ClientId};

pub(crate) fn next_grapheme(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.get_win_buf_mut(id);
    let cursor = win.primary_cursor_mut();
    let pos = common::movement::next_grapheme_boundary(&buf.slice(..), cursor.pos());
    cursor.goto(pos);
}

pub(crate) fn prev_grapheme(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.get_win_buf_mut(id);
    let cursor = win.primary_cursor_mut();
    let pos = common::movement::prev_grapheme_boundary(&buf.slice(..), cursor.pos());
    cursor.goto(pos);
}

pub(crate) fn start_of_line(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.get_win_buf_mut(id);
    let cursor = win.primary_cursor_mut();
    let pos = common::movement::start_of_line(&buf.slice(..), cursor.pos());
    cursor.goto(pos);
}

pub(crate) fn end_of_line(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.get_win_buf_mut(id);
    let cursor = win.primary_cursor_mut();
    let pos = common::movement::end_of_line(&buf.slice(..), cursor.pos());
    cursor.goto(pos);
}

pub(crate) fn start_of_buffer(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.get_win_buf_mut(id);
    let cursor = win.primary_cursor_mut();
    cursor.goto(0);
}

pub(crate) fn end_of_buffer(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.get_win_buf_mut(id);
    let cursor = win.primary_cursor_mut();
    cursor.goto(buf.len());
}

pub(crate) fn prev_visual_line(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.get_win_buf_mut(id);
    let cursor_point = win.view().primary_cursor();
    let on_first_line = cursor_point.y == 0;
    if on_first_line {
        win.scroll_up(buf);
    }

    let line = cursor_point.y.saturating_sub(1);
    let cursor_col = win.primary_cursor().column().unwrap_or(cursor_point.x);
    let max_col = win.view().last_char_on_line(line).x;
    let col = cmp::min(max_col, cursor_col);

    if let Some(pos) = win.view().pos_at_point(Point { x: col, y: line }) {
        win.primary_cursor_mut().goto_with_col(pos, cursor_col);
    }
}

pub(crate) fn next_visual_line(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.get_win_buf_mut(id);
    let cursor_point = win.view().primary_cursor();
    let last_line = win.view().height().saturating_sub(1);
    let on_last_line = cursor_point.y == last_line;
    if on_last_line {
        win.scroll_down(buf);
    }

    let line = cmp::min(last_line, cursor_point.y + 1);
    let max_col = win.view().last_char_on_line(line).x;
    let cursor_col = win.primary_cursor().column().unwrap_or(cursor_point.x);
    let col = cmp::min(max_col, cursor_col);

    if let Some(pos) = win.view().pos_at_point(Point { x: col, y: line }) {
        win.primary_cursor_mut().goto_with_col(pos, cursor_col);
    }
}
