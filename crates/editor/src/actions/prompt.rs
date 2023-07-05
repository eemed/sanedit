use std::{path::PathBuf, rc::Rc};

use crate::{
    editor::{
        windows::{Focus, Prompt, PromptAction},
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

pub(crate) fn prompt_open_file(editor: &mut Editor, id: ClientId) {
    let job_id = jobs::list_files_provide_completions(editor, id);
    let on_confirm: PromptAction = Rc::new(move |editor, id, input| {
        editor.jobs.stop(&job_id);
        let path = PathBuf::from(input);
        if editor.open_file(id, path).is_err() {
            let (win, _buf) = editor.win_buf_mut(id);
            win.warn_msg("Failed to open file");
        }
    });
    let on_abort: PromptAction = Rc::new(move |editor, _id, _input| {
        editor.jobs.stop(&job_id);
    });

    let (win, _buf) = editor.win_buf_mut(id);
    win.prompt = Prompt::new("Open a file");
    win.prompt.on_confirm = Some(on_confirm);
    win.prompt.on_abort = Some(on_abort);
    win.focus = Focus::Prompt;
}

pub(crate) fn prompt_close(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    if let Some(on_abort) = win.prompt.on_abort.clone() {
        let input = win.prompt.input_or_selected();
        (on_abort)(editor, id, &input)
    }

    let (win, _buf) = editor.win_buf_mut(id);
    win.focus = Focus::Window;
}

pub(crate) fn prompt_confirm(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    if let Some(on_confirm) = win.prompt.on_confirm.clone() {
        let input = win.prompt.input_or_selected();
        win.prompt.history.push(&input);
        (on_confirm)(editor, id, &input)
    }

    let (win, _buf) = editor.win_buf_mut(id);
    win.focus = Focus::Window;
}

pub(crate) fn prompt_next_grapheme(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.prompt.next_grapheme();
}

pub(crate) fn prompt_prev_grapheme(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.prompt.prev_grapheme();
}

pub(crate) fn prompt_remove_grapheme_before_cursor(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.prompt.remove_grapheme_before_cursor();

    if let Some(on_input) = win.prompt.on_input.clone() {
        let input = win.prompt.input().to_string();
        (on_input)(editor, id, &input)
    }
}

pub(crate) fn prompt_next_completion(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.prompt.next_completion();
}

pub(crate) fn prompt_prev_completion(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.prompt.prev_completion();
}

pub(crate) fn provide_completions(editor: &mut Editor, id: ClientId, completions: Vec<String>) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.prompt.provide_completions(completions);
}

pub(crate) fn prompt_history_next(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.prompt.history_next();
}

pub(crate) fn prompt_history_prev(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.prompt.history_prev();
}
