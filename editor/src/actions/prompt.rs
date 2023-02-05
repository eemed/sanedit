use crate::{editor::Editor, server::ClientId};

pub(crate) fn prompt_next_grapheme(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.get_win_buf_mut(id);
    if let Some(prompt) = win.prompt_mut() {
        prompt.next_grapheme();
    }
}

pub(crate) fn prompt_prev_grapheme(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.get_win_buf_mut(id);
    if let Some(prompt) = win.prompt_mut() {
        prompt.prev_grapheme();
    }
}

pub(crate) fn prompt_insert_at_cursor(editor: &mut Editor, id: ClientId, string: &str) {
    let (win, buf) = editor.get_win_buf_mut(id);
    if let Some(prompt) = win.prompt_mut() {
        prompt.insert_at_cursor(string);
    }
}

pub(crate) fn prompt_insert_char_at_cursor(editor: &mut Editor, id: ClientId, ch: char) {
    let (win, buf) = editor.get_win_buf_mut(id);
    if let Some(prompt) = win.prompt_mut() {
        prompt.insert_char_at_cursor(ch);
    }
}

pub(crate) fn prompt_remove_grapheme_at_cursor(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.get_win_buf_mut(id);
    if let Some(prompt) = win.prompt_mut() {
        prompt.remove_grapheme_at_cursor();
    }
}

pub(crate) fn confirm(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.get_win_buf_mut(id);
    if let Some(prompt) = win.prompt_take() {
        let action = prompt.action();
        let input = prompt.input();
        (action)(editor, id, input)
    }
}

pub(crate) fn prompt_next_completion(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.get_win_buf_mut(id);
    if let Some(prompt) = win.prompt_mut() {
        prompt.next_completion();
    }
}

pub(crate) fn prompt_prev_completion(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.get_win_buf_mut(id);
    if let Some(prompt) = win.prompt_mut() {
        prompt.next_completion();
    }
}

pub(crate) fn provide_completions(editor: &mut Editor, id: ClientId, completions: Vec<String>) {
    let (win, buf) = editor.get_win_buf_mut(id);
    if let Some(prompt) = win.prompt_mut() {
        prompt.provide_completions(completions);
    }
}
