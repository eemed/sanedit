use std::{path::PathBuf, rc::Rc};
use tokio::sync::mpsc::channel;

// use super::jobs::{self, Matches};
use crate::{
    actions::jobs::OpenFile,
    common::matcher::Match,
    editor::{
        windows::{Focus, Prompt},
        Editor,
    },
    server::ClientId,
};

#[action("Open a file")]
fn open_file(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);

    win.prompt = Prompt::builder()
        .prompt("Open a file")
        .on_confirm(move |editor, id, input| {
            let path = PathBuf::from(input);

            if let Err(e) = editor.open_file(id, &path) {
                let (win, _buf) = editor.win_buf_mut(id);
                win.warn_msg(&format!("Failed to open file {input}"))
            }
        })
        .build();
    win.focus = Focus::Prompt;

    let path = editor.working_dir().to_path_buf();
    let job = OpenFile::new(id, path);
    editor.job_broker.request(job);
}

#[action("Close prompt")]
fn close(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    if let Some(on_abort) = win.prompt.on_abort() {
        let input = win.prompt.input_or_selected();
        (on_abort)(editor, id, &input)
    }

    let (win, _buf) = editor.win_buf_mut(id);
    win.focus = Focus::Window;
}

#[action("Confirm selection")]
fn confirm(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    if let Some(on_confirm) = win.prompt.on_confirm() {
        win.prompt.save_to_history();
        let input = win.prompt.input_or_selected();
        (on_confirm)(editor, id, &input)
    }

    let (win, _buf) = editor.win_buf_mut(id);
    win.focus = Focus::Window;
}

#[action("Move cursor one character right")]
fn next_grapheme(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.prompt.next_grapheme();
}

#[action("Move cursor one character left")]
fn prev_grapheme(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.prompt.prev_grapheme();
}

#[action("Delete a character before cursor")]
pub(crate) fn remove_grapheme_before_cursor(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.prompt.remove_grapheme_before_cursor();

    if let Some(on_input) = win.prompt.on_input() {
        let input = win.prompt.input().to_string();
        (on_input)(editor, id, &input)
    }
}

#[action("Select the next completion item")]
fn next_completion(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.prompt.next_completion();
}

#[action("Select the previous completion item")]
fn prev_completion(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.prompt.prev_completion();
}

// pub(crate) fn provide_completions(editor: &mut Editor, id: ClientId, completions: Matches) {
//     let (win, _buf) = editor.win_buf_mut(id);
//     win.prompt.provide_completions(completions);
// }

#[action("Select the next entry from history")]
fn history_next(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.prompt.history_next();
}

#[action("Select the previous entry from history")]
fn history_prev(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.prompt.history_prev();
}
