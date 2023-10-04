use std::{io, path::PathBuf, rc::Rc};

use crate::{
    editor::{
        hooks::Hook,
        windows::{Focus, Prompt},
        Editor,
    },
    server::ClientId,
};

use super::{hooks::run, Action};

#[action("Remove character after cursor")]
fn remove_grapheme_after_cursor(editor: &mut Editor, id: ClientId) {
    run(editor, id, Hook::RemoveCharPre);
    let (win, buf) = editor.win_buf_mut(id);
    win.remove_grapheme_after_cursors(buf);
}

#[action("Remove character before cursor")]
fn remove_grapheme_before_cursor(editor: &mut Editor, id: ClientId) {
    run(editor, id, Hook::RemoveCharPre);
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
            if let Some(on_input) = win.search.prompt.get_on_input() {
                let input = win.search.prompt.input_or_selected();
                (on_input)(editor, id, &input)
            }
        }
        Focus::Prompt => {
            win.prompt.insert_at_cursor(text);
            if let Some(on_input) = win.prompt.get_on_input() {
                let input = win.prompt.input_or_selected();
                (on_input)(editor, id, &input)
            }
        }
        Focus::Window => {
            run(editor, id, Hook::InsertCharPre);
            let (win, buf) = editor.win_buf_mut(id);
            win.insert_at_cursors(buf, text);
        }
        Focus::Completion => {}
    }
}

#[action("Save file")]
fn save(editor: &mut Editor, id: ClientId) {
    // let (_win, buf) = editor.win_buf_mut(id);
    // if buf.path().is_none() {
    //     save_as.execute(editor, id);
    //     return;
    // }

    // match jobs::save_file(editor, id) {
    //     Ok(job) => {
    //         editor.jobs.request(job);
    //     }
    //     Err(e) => {
    //         let (win, buf) = editor.win_buf_mut(id);
    //         win.error_msg(&format!("Failed to save buffer {}, {e:?}", buf.name()));
    //     }
    // }
}

#[action("Prompt filename and save file")]
fn save_as(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.prompt = Prompt::new("Save as");
    win.prompt.on_confirm = Some(Rc::new(|editor, id, path| {
        let (_win, buf) = editor.win_buf_mut(id);
        buf.set_path(PathBuf::from(path));
        save.execute(editor, id);
    }));
    win.focus = Focus::Prompt;
}
