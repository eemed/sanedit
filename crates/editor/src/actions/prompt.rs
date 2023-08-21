use std::{path::PathBuf, rc::Rc};

use tokio::sync::mpsc::channel;

use crate::{
    common::matcher::Match,
    editor::{
        windows::{Focus, Prompt},
        Editor,
    },
    server::ClientId,
};

use super::jobs;

fn is_yes(input: &str) -> bool {
    match input {
        "y" | "Y" | "yes" => true,
        _ => false,
    }
}

#[action("Open a file")]
fn open_file(editor: &mut Editor, id: ClientId) {
    const CHANNEL_SIZE: usize = 64;

    let (tx, rx) = channel(CHANNEL_SIZE);
    let job = jobs::list_files(editor, id, rx);

    let (win, _buf) = editor.win_buf_mut(id);
    win.prompt = Prompt::new("Open a file");
    win.prompt.on_input = Some(Rc::new(move |editor, id, input| {
        let _ = tx.blocking_send(input.into());
    }));
    win.prompt.on_confirm = Some(Rc::new(move |editor, id, input| {
        let (win, _buf) = editor.win_buf_mut(id);
        win.prompt.on_input = None;
        let path = PathBuf::from(input);

        if let Err(e) = editor.open_file(id, &path) {
            let (win, _buf) = editor.win_buf_mut(id);
            win.warn_msg(&format!("Failed to open file {input}"))
        }
    }));
    win.prompt.on_abort = Some(Rc::new(move |editor, id, input| {
        let (win, _buf) = editor.win_buf_mut(id);
        win.prompt.on_input = None;
    }));
    win.focus = Focus::Prompt;

    editor.jobs.request(job);
}

#[action("Close prompt")]
fn close(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    if let Some(on_abort) = win.prompt.on_abort.clone() {
        let input = win.prompt.input_or_selected();
        (on_abort)(editor, id, &input)
    }

    let (win, _buf) = editor.win_buf_mut(id);
    win.focus = Focus::Window;
}

#[action("Confirm selection")]
fn confirm(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    if let Some(on_confirm) = win.prompt.on_confirm.clone() {
        let input = win.prompt.input_or_selected();
        win.prompt.history.push(&input);
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

    if let Some(on_input) = win.prompt.on_input.clone() {
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

pub(crate) fn provide_completions(editor: &mut Editor, id: ClientId, completions: Vec<Match>) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.prompt.provide_completions(completions);
}

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
