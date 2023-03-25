use crate::{common, editor::Editor, server::ClientId};

pub(crate) fn remove_grapheme_after_cursor(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.get_win_buf_mut(id);
    let cursor = win.primary_cursor_mut();
    let pos = common::movement::next_grapheme_boundary(&buf.slice(..), cursor.pos());
    buf.remove(cursor.pos()..pos);
}

pub(crate) fn remove_grapheme_before_cursor(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.get_win_buf_mut(id);
    let cursor = win.primary_cursor_mut();
    let pos = common::movement::prev_grapheme_boundary(&buf.slice(..), cursor.pos());
    buf.remove(pos..cursor.pos());
    cursor.goto(pos);
}

pub(crate) fn undo(editor: &mut Editor, id: ClientId) {}

pub(crate) fn redo(editor: &mut Editor, id: ClientId) {}
