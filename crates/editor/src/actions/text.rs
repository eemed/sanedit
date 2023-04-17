use crate::{
    editor::{windows::Focus, Editor},
    server::ClientId,
};

pub(crate) fn remove_grapheme_after_cursor(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    win.remove_grapheme_after_cursor(buf);
}

pub(crate) fn remove_grapheme_before_cursor(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    win.remove_grapheme_before_cursor(buf);
}

pub(crate) fn undo(editor: &mut Editor, id: ClientId) {}

pub(crate) fn redo(editor: &mut Editor, id: ClientId) {}

pub(crate) fn insert(editor: &mut Editor, id: ClientId, text: &str) {
    let (win, buf) = editor.win_buf_mut(id);

    match win.focus() {
        Focus::Search => {
            win.search.prompt.insert_at_cursor(text);
            if let Some(on_input) = win.search.prompt.on_input.clone() {
                let input = win.search.prompt.input();
                (on_input)(editor, id, &input)
            }
        }
        Focus::Prompt => {
            win.prompt.insert_at_cursor(text);
            if let Some(on_input) = win.prompt.on_input.clone() {
                let input = win.prompt.input();
                (on_input)(editor, id, &input)
            }
        }
        Focus::Window => {
            win.insert_at_cursor(buf, text);
        }
    }
}
