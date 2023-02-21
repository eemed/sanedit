use std::{mem, path::PathBuf, sync::Arc};

use crate::{
    common::file::FileMetadata,
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

pub(crate) fn prompt_file_conversion(editor: &mut Editor, id: ClientId, file: FileMetadata) {
    let action: PromptAction = Box::new(|editor, id, input| {
        let yes = is_yes(input);
        if yes {
            log::info!("TODO do convert")
        }
    });
    let msg = format!(
        "File {} is not UTF-8, Convert? (Y/n)",
        file.path
            .file_name()
            .expect("cannot get filename")
            .to_string_lossy()
    );
    let prompt = Prompt::new(&msg, action, false);
    let (win, buf) = editor.get_win_buf_mut(id);
    win.open_prompt(prompt);
}

pub(crate) fn prompt_open_file(editor: &mut Editor, id: ClientId) {
    let action: PromptAction = Box::new(|editor, id, input| {
        let path = PathBuf::from(input);
        editor.open_file(id, path);
    });
    let prompt = Prompt::new("Open a file", action, false);
    let (win, buf) = editor.get_win_buf_mut(id);
    win.open_prompt(prompt);

    jobs::list_files_provide_completions(editor, id);
}

pub(crate) fn prompt_close(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.get_win_buf_mut(id);
    win.close_prompt();
}

pub(crate) fn prompt_next_grapheme(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.get_win_buf_mut(id);
    win.prompt.next_grapheme();
}

pub(crate) fn prompt_prev_grapheme(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.get_win_buf_mut(id);
    win.prompt.prev_grapheme();
}

pub(crate) fn prompt_insert_at_cursor(editor: &mut Editor, id: ClientId, string: &str) {
    let (win, buf) = editor.get_win_buf_mut(id);
    win.prompt.insert_at_cursor(string);
}

pub(crate) fn prompt_insert_char_at_cursor(editor: &mut Editor, id: ClientId, ch: char) {
    let (win, buf) = editor.get_win_buf_mut(id);
    win.prompt.insert_char_at_cursor(ch);
}

pub(crate) fn prompt_remove_grapheme_after_cursor(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.get_win_buf_mut(id);
    win.prompt.remove_grapheme_after_cursor();
}

pub(crate) fn prompt_confirm(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.get_win_buf_mut(id);
    let prompt = mem::take(&mut win.prompt);
    win.close_prompt();
    prompt.execute_action(editor, id);
}

pub(crate) fn prompt_next_completion(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.get_win_buf_mut(id);
    win.prompt.next_completion();
}

pub(crate) fn prompt_prev_completion(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.get_win_buf_mut(id);
    win.prompt.prev_completion();
}

pub(crate) fn provide_completions(editor: &mut Editor, id: ClientId, completions: Vec<String>) {
    let (win, buf) = editor.get_win_buf_mut(id);
    win.prompt.provide_completions(completions);
}
