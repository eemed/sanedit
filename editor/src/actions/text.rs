use crate::{editor::Editor, server::ClientId};

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

pub(crate) fn remove_char_after_cursor(editor: &mut Editor, id: ClientId) {}

pub(crate) fn remove_char_before_cursor(editor: &mut Editor, id: ClientId) {

    // let edit = {
    //     let editor = state.borrow();
    //     let (win, buf) = editor.win_buf();
    //     let cursor = &win.cursor;
    //     let pos = {
    //         let mut graphemes = buf.graphemes_at(cursor.pos());
    //         graphemes.prev();
    //         graphemes.pos()
    //     };
    //     let range = pos..cursor.pos();
    //     Edit::Remove {
    //         range: range.clone(),
    //     }
    // };

    // if let Err(e) = insert(state, edit) {
    //     state.borrow_mut().warn_msg(&e.to_string());
    // }
}

pub(crate) fn undo(editor: &mut Editor, id: ClientId) {}

pub(crate) fn redo(editor: &mut Editor, id: ClientId) {}

fn insert() {}

fn remove() {}
