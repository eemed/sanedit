use std::{path::PathBuf, rc::Rc};

use crate::{
    editor::{
        windows::{Focus, PAction, SetPrompt},
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
    let on_confirm: PAction = Rc::new(move |editor, id, input| {
        editor.jobs.stop(&job_id);
        let path = PathBuf::from(input);
        if editor.open_file(id, path).is_err() {
            let (win, _buf) = editor.win_buf_mut(id);
            // TODO clear messages, somewhere
            // win.warn_msg("Failed to open file".into());
        }
    });
    let on_abort: PAction = Rc::new(move |editor, id, input| {
        editor.jobs.stop(&job_id);
    });

    let set = SetPrompt {
        message: "Open a file".into(),
        on_confirm: Some(on_confirm),
        on_abort: Some(on_abort),
        on_input: None,
        keymap: None,
    };

    let (win, buf) = editor.win_buf_mut(id);
    win.prompt.set(set);
    win.focus = Focus::Prompt;
}

pub(crate) fn prompt_close(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    if let Some(on_abort) = win.prompt.on_abort.clone() {
        let input = win.prompt.input();
        (on_abort)(editor, id, &input)
    }

    let (win, buf) = editor.win_buf_mut(id);
    win.focus = Focus::Window;
}

pub(crate) fn prompt_confirm(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    if let Some(on_confirm) = win.prompt.on_confirm.clone() {
        let input = win.prompt.input();
        win.prompt.history.push(&input);
        (on_confirm)(editor, id, &input)
    }

    let (win, buf) = editor.win_buf_mut(id);
    win.focus = Focus::Window;
}

pub(crate) fn prompt_next_grapheme(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    win.prompt.next_grapheme();
}

pub(crate) fn prompt_prev_grapheme(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    win.prompt.prev_grapheme();
}

pub(crate) fn prompt_remove_grapheme_before_cursor(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    win.prompt.remove_grapheme_before_cursor();

    if let Some(on_input) = win.prompt.on_input.clone() {
        let input = win.prompt.input();
        (on_input)(editor, id, &input)
    }
}

pub(crate) fn prompt_next_completion(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    win.prompt.next_completion();
}

pub(crate) fn prompt_prev_completion(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    win.prompt.prev_completion();
}

pub(crate) fn provide_completions(editor: &mut Editor, id: ClientId, completions: Vec<String>) {
    let (win, buf) = editor.win_buf_mut(id);
    win.prompt.provide_completions(completions);
}

pub(crate) fn prompt_history_next(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    win.prompt.history_next();
}

pub(crate) fn prompt_history_prev(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    win.prompt.history_prev();
}
