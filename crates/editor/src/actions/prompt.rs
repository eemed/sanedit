use std::{path::PathBuf, rc::Rc};

use crate::{
    editor::{
        windows::{Prompt, PromptAction},
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
        editor.jobs_mut().stop(&job_id);
        let path = PathBuf::from(input);
        if editor.open_file(id, path).is_err() {
            let (win, _buf) = editor.win_buf_mut(id);
            // TODO clear messages, somewhere
            // win.warn_msg("Failed to open file".into());
        }
    });

    let on_abort: PromptAction = Rc::new(move |editor, id, input| {
        editor.jobs_mut().stop(&job_id);
    });
    let prompt = Prompt::new("Open a file")
        .on_confirm(on_confirm)
        .on_abort(on_abort);
    let (win, buf) = editor.win_buf_mut(id);
    win.open_prompt(prompt);
}

pub(crate) fn prompt_close(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    win.close_prompt().abort(editor, id);
}

pub(crate) fn prompt_confirm(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    win.close_prompt().confirm(editor, id);
}

pub(crate) fn prompt_next_grapheme(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    win.prompt_mut().next_grapheme();
}

pub(crate) fn prompt_prev_grapheme(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    win.prompt_mut().prev_grapheme();
}

pub(crate) fn prompt_remove_grapheme_before_cursor(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    win.prompt_mut().remove_grapheme_before_cursor();

    if let Some((on_input, input)) = win.prompt_mut().get_on_input() {
        (on_input)(editor, id, &input);
    }
}

pub(crate) fn prompt_next_completion(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    win.prompt_mut().next_completion();
}

pub(crate) fn prompt_prev_completion(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    win.prompt_mut().prev_completion();
}

pub(crate) fn provide_completions(editor: &mut Editor, id: ClientId, completions: Vec<String>) {
    let (win, buf) = editor.win_buf_mut(id);
    win.prompt_mut().provide_completions(completions);
}
