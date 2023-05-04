use sanedit_messages::redraw::Point;

use crate::{
    editor::{windows::Cursor, Editor},
    server::ClientId,
};

pub(crate) fn new_cursor_below(editor: &mut Editor, id: ClientId) {
}

pub(crate) fn new_cursor_above(editor: &mut Editor, id: ClientId) {
}

pub(crate) fn new_cursor_to_point(editor: &mut Editor, id: ClientId, point: Point) {
    let (win, buf) = editor.win_buf_mut(id);
    if let Some(pos) = win.view().pos_at_point(point) {
        win.cursors.push(Cursor::new(pos));
    }
}

pub(crate) fn remove_secondary_cursors(editor: &mut Editor, id: ClientId) {}

pub(crate) fn start_selection(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    win.cursors.start_selection();
}
