use std::{fs::File, mem, path::PathBuf, sync::Arc};

use crate::{
    editor::{
        buffers::Buffer,
        windows::{Prompt, PromptAction},
        Editor,
    },
    server::ClientId,
};

use super::jobs;

pub(crate) fn prompt_open_file(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.get_win_buf_mut(id);
    let action: PromptAction = Arc::new(|editor, id, input| {
        let path = PathBuf::from(input);
        if !path.is_file() {
            log::error!("File {input} is not a file");
            return;
        }
        let file = File::open(&path).expect("Failed to open file {input}");
        let buf = Buffer::from_reader(file).expect("Failed to read file {input}");
        editor.open_new_buffer(id, buf);
    });
    let prompt = Prompt::new("Open a file", action, false);
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

    let action = prompt.action();
    let input = prompt
        .selected()
        .map(|(_, item)| item)
        .unwrap_or(prompt.input());
    (action)(editor, id, input)
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
