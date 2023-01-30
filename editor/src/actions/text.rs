use crate::{editor::Editor, server::ClientId, common};

pub(crate) fn insert_char_at_cursor(editor: &mut Editor, id: ClientId, ch: char) {
    let mut buf = [0u8; 4];
    let string = ch.encode_utf8(&mut buf);
    insert_at_cursor(editor, id, string);
}

pub(crate) fn insert_at_cursor<B: AsRef<[u8]>>(editor: &mut Editor, id: ClientId, bytes: B) {
    fn inner(editor: &mut Editor, id: ClientId, bytes: &[u8]) {
        let (win, buf) = editor.get_win_buf_mut(id);
        let cursor = win.primary_cursor_mut();
        let cursor_pos = cursor.pos();
        buf.insert(cursor_pos, bytes);
        cursor.goto(cursor_pos + bytes.len());
    }

    inner(editor, id, bytes.as_ref());
}

pub(crate) fn remove_char_after_cursor(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.get_win_buf_mut(id);
    let cursor = win.primary_cursor_mut();
    let pos = common::movement::next_grapheme_boundary(&buf.slice(..), cursor.pos());
    buf.remove(cursor.pos()..pos);
}

pub(crate) fn remove_char_before_cursor(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.get_win_buf_mut(id);
    let cursor = win.primary_cursor_mut();
    let pos = common::movement::prev_grapheme_boundary(&buf.slice(..), cursor.pos());
    buf.remove(pos..cursor.pos());
    cursor.goto(pos);
}

pub(crate) fn undo(editor: &mut Editor, id: ClientId) {}

pub(crate) fn redo(editor: &mut Editor, id: ClientId) {}
