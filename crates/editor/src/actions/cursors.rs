use sanedit_messages::redraw::Point;

use crate::{
    common::{
        movement::{next_line, prev_line},
        search::{search, search_all},
    },
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

pub(crate) fn new_cursor_to_next_search_match(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    let last_search = win.search.prompt.input();
    let ppos = win.cursors.primary().pos();

    if let Some(mut mat) = search(last_search.as_bytes(), &buf.slice(ppos..)) {
        mat.start += ppos;
        mat.end += ppos;

        let selecting = win.primary_cursor().selection().is_none();
        if selecting {
            let cursor = win.cursors.primary_mut();
            *cursor = Cursor::new_select(&mat);
        } else {
            win.cursors.push_primary(Cursor::new_select(&mat));
        }
    }
}

pub(crate) fn new_cursors_to_all_search_matches(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    let last_search = win.search.prompt.input();
    let ppos = win.cursors.primary().pos();

    if let Some(mut mat) = search_all(last_search.as_bytes(), &buf.slice(ppos..)) {
        mat.start += ppos;
        mat.end += ppos;

        let selecting = win.primary_cursor().selection().is_none();
        if selecting {
            let cursor = win.cursors.primary_mut();
            *cursor = Cursor::new_select(&mat);
        } else {
            win.cursors.push_primary(Cursor::new_select(&mat));
        }
    }
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

pub(crate) fn swap_selection_dir(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    win.cursors.swap_selection_dir();
}
