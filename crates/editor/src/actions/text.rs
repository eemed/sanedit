use crate::{
    editor::{hooks::Hook, windows::Focus, Editor},
    server::ClientId,
};

use super::hooks::execute;

#[action("Remove character after cursor")]
fn remove_grapheme_after_cursor(editor: &mut Editor, id: ClientId) {
    execute(editor, id, Hook::RemoveCharPre);
    let (win, buf) = editor.win_buf_mut(id);
    win.remove_grapheme_after_cursors(buf);
}

#[action("Remove character before cursor")]
fn remove_grapheme_before_cursor(editor: &mut Editor, id: ClientId) {
    execute(editor, id, Hook::RemoveCharPre);
    let (win, buf) = editor.win_buf_mut(id);
    win.remove_grapheme_before_cursors(buf);
}

#[action("Undo a change")]
pub(crate) fn undo(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    win.undo(buf);
}

#[action("Redo a change")]
pub(crate) fn redo(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    win.redo(buf);
}

pub(crate) fn insert(editor: &mut Editor, id: ClientId, text: &str) {
    let (win, _buf) = editor.win_buf_mut(id);

    match win.focus() {
        Focus::Search => {
            win.search.prompt.insert_at_cursor(text);
            if let Some(on_input) = win.search.prompt.on_input.clone() {
                let input = win.search.prompt.input_or_selected();
                (on_input)(editor, id, &input)
            }
        }
        Focus::Prompt => {
            win.prompt.insert_at_cursor(text);
            if let Some(on_input) = win.prompt.on_input.clone() {
                let input = win.prompt.input_or_selected();
                (on_input)(editor, id, &input)
            }
        }
        Focus::Window => {
            execute(editor, id, Hook::InsertCharPre);
            let (win, buf) = editor.win_buf_mut(id);
            win.insert_at_cursors(buf, text);
        }
        Focus::Completion => {}
    }
}

#[action("Save file")]
fn save(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    if let Err(e) = buf.save() {
        win.error_msg(&format!("Saving failed: {e}"));
        log::error!("Failed to save buffer {}", e);
    }
}
