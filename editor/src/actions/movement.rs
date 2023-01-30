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
    let (win, buf) = editor.get_win_buf_mut(id);
    let cursor = win.primary_cursor_mut();
    cursor.goto(0);
}

pub(crate) fn end_of_buffer(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.get_win_buf_mut(id);
    let cursor = win.primary_cursor_mut();
    cursor.goto(buf.len());
}
