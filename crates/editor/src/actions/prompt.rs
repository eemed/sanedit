mod commands;

use std::{mem, path::PathBuf};

use crate::{
    actions::{jobs::OpenFile, prompt::commands::find_action},
    editor::{
        windows::{Focus, Prompt},
        Editor,
    },
    server::ClientId,
};

use super::jobs::{ShellCommand, StaticMatcher};

#[action("Command palette")]
fn command_palette(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);

    let job = StaticMatcher::new(id, commands::command_palette(), commands::format_match);

    win.prompt = Prompt::builder()
        .prompt("Command")
        .on_confirm(move |editor, id, input| match find_action(input) {
            Some(action) => action.execute(editor, id),
            None => log::error!("No action with name {input}"),
        })
        .build();
    win.focus = Focus::Prompt;

    editor.job_broker.request(job);
}

#[action("Open a file")]
fn open_file(editor: &mut Editor, id: ClientId) {
    let path = editor.working_dir().to_path_buf();
    let job = OpenFile::new(id, path);
    let jid = editor.job_broker.request(job);
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
        .background_job(jid)
        .build();
    win.focus = Focus::Prompt;
}

#[action("Close prompt")]
fn close(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.focus = Focus::Window;

    if let Some(on_abort) = win.prompt.on_abort() {
        let input = win.prompt.input_or_selected();
        (on_abort)(editor, id, &input)
    }

    let (win, _buf) = editor.win_buf_mut(id);
    if let Some(id) = win.prompt.background_job() {
        editor.job_broker.stop(id);
    }
}

#[action("Confirm selection")]
fn confirm(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.focus = Focus::Window;

    if let Some(on_confirm) = win.prompt.on_confirm() {
        win.prompt.save_to_history();
        let input = win.prompt.input_or_selected();
        (on_confirm)(editor, id, &input)
    }

    let (win, _buf) = editor.win_buf_mut(id);
    if let Some(id) = win.prompt.background_job() {
        editor.job_broker.stop(id);
    }
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

#[action("Run a shell command")]
fn shell_command(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);

    win.prompt = Prompt::builder()
        .prompt("Command")
        .on_confirm(move |editor, id, input| {
            let job = ShellCommand::new(id, input);
            editor.job_broker.request(job);
        })
        .build();
    win.focus = Focus::Prompt;
}
