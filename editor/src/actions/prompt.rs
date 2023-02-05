use crate::{editor::Editor, server::ClientId};

pub(crate) fn prompt_open_file(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.get_win_buf_mut(id);
    tokio::spawn(async {
    });
    // win.open_prompt();
}

pub(crate) fn prompt_close(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.get_win_buf_mut(id);
    win.close_prompt();
}

pub(crate) fn prompt_next_grapheme(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.get_win_buf_mut(id);
    win.prompt_mut().next_grapheme();
}

pub(crate) fn prompt_prev_grapheme(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.get_win_buf_mut(id);
    win.prompt_mut().prev_grapheme();
}

pub(crate) fn prompt_insert_at_cursor(editor: &mut Editor, id: ClientId, string: &str) {
    let (win, buf) = editor.get_win_buf_mut(id);
    win.prompt_mut().insert_at_cursor(string);
}

pub(crate) fn prompt_insert_char_at_cursor(editor: &mut Editor, id: ClientId, ch: char) {
    let (win, buf) = editor.get_win_buf_mut(id);
    win.prompt_mut().insert_char_at_cursor(ch);
}

pub(crate) fn prompt_remove_grapheme_after_cursor(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.get_win_buf_mut(id);
    win.prompt_mut().remove_grapheme_after_cursor();
}

pub(crate) fn prompt_confirm(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.get_win_buf_mut(id);
    let prompt = win.take_prompt();
    let action = prompt.action();
    let input = prompt.input();
    (action)(editor, id, input)
}

pub(crate) fn prompt_next_completion(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.get_win_buf_mut(id);
    win.prompt_mut().next_completion();
}

pub(crate) fn prompt_prev_completion(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.get_win_buf_mut(id);
    win.prompt_mut().prev_completion();
}

pub(crate) fn provide_completions(editor: &mut Editor, id: ClientId, completions: Vec<String>) {
    let (win, buf) = editor.get_win_buf_mut(id);
    win.prompt_mut().provide_completions(completions);
}
