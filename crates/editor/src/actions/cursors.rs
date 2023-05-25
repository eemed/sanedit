use sanedit_messages::redraw::Point;

use crate::{
    common::movement::{next_line, prev_line},
    editor::{windows::Cursor, Editor},
    server::ClientId,
};

pub(crate) fn new_cursor_next_line(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    let cursor = win.cursors.primary();
    let (pos, _col) = next_line(&buf.slice(..), cursor, win.display_options());
    win.cursors.push_primary(Cursor::new(pos));
}

pub(crate) fn new_cursor_prev_line(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    let cursor = win.cursors.primary();
    let (pos, _col) = prev_line(&buf.slice(..), cursor, win.display_options());
    win.cursors.push_primary(Cursor::new(pos));
}

pub(crate) fn new_cursor_to_point(editor: &mut Editor, id: ClientId, point: Point) {
    let (win, buf) = editor.win_buf_mut(id);
    if let Some(pos) = win.view().pos_at_point(point) {
        win.cursors.push(Cursor::new(pos));
    }
}

pub(crate) fn start_selection(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    win.cursors.start_selection();
}

pub(crate) fn goto_position(editor: &mut Editor, id: ClientId, point: Point) {
    let (win, buf) = editor.win_buf_mut(id);
    win.cursors.remove_secondary_cursors();
    if let Some(pos) = win.view().pos_at_point(point) {
        win.cursors.primary_mut().goto(pos);
    }
}

pub(crate) fn remove_secondary_cursors(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    win.cursors.remove_secondary_cursors();
}
