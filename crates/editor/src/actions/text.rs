use std::{io, path::PathBuf, rc::Rc};

use sanedit_buffer::PieceTree;

use crate::{
    common::dirs::{tmp_dir, tmp_file},
    editor::{
        hooks::Hook,
        windows::{Focus, Prompt},
        Editor,
    },
    server::ClientId,
};

use super::{hooks::run, jobs, Action};

#[action("Remove character after cursor")]
fn remove_grapheme_after_cursor(editor: &mut Editor, id: ClientId) {
    run(editor, id, Hook::RemovePre);
    let (win, buf) = editor.win_buf_mut(id);
    win.remove_grapheme_after_cursors(buf);
}

#[action("Remove character before cursor")]
fn remove_grapheme_before_cursor(editor: &mut Editor, id: ClientId) {
    run(editor, id, Hook::RemovePre);
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
            if let Some(on_input) = win.search.prompt.on_input() {
                let input = win.search.prompt.input_or_selected();
                (on_input)(editor, id, &input)
            }
        }
        Focus::Prompt => {
            win.prompt.insert_at_cursor(text);
            if let Some(on_input) = win.prompt.on_input() {
                let input = win.prompt.input_or_selected();
                (on_input)(editor, id, &input)
            }
        }
        Focus::Window => {
            run(editor, id, Hook::InsertPre);
            let (win, buf) = editor.win_buf_mut(id);
            win.insert_at_cursors(buf, text);
        }
        Focus::Completion => {}
    }
}

#[action("Save file")]
fn save(editor: &mut Editor, id: ClientId) {
    let (_win, buf) = editor.win_buf_mut(id);
    if buf.path().is_none() {
        save_as.execute(editor, id);
        return;
    }

    let ropt = buf.read_only_copy();
    let target = match tmp_file() {
        Some(tmp) => tmp,
        None => return,
    };

    let (_win, buf) = editor.win_buf_mut(id);
    buf.start_saving();
    let job = jobs::Save::new(id, ropt, target);
    editor.job_broker.request(job);
}

#[action("Prompt filename and save file")]
fn save_as(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.prompt = Prompt::builder()
        .prompt("Save as")
        .on_confirm(|editor, id, path| {
            let (_win, buf) = editor.win_buf_mut(id);
            buf.set_path(PathBuf::from(path));
            save.execute(editor, id);
        })
        .build();
    win.focus = Focus::Prompt;
}

#[action("Copy selection to clipboard")]
fn copy(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    win.copy_to_clipboard(buf);
}

#[action("Paste from clipboard")]
fn paste(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    win.paste_from_clipboard(buf);
}
