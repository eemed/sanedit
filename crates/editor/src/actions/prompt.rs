mod commands;

use std::{cmp::min, path::PathBuf, sync::Arc};

use sanedit_messages::ClientMessage;

use crate::{
    actions::jobs::FileOptionProvider,
    editor::{
        hooks::Hook,
        windows::{Focus, Prompt},
        Editor,
    },
    server::ClientId,
};

use self::commands::find_action;

use super::{
    hooks,
    jobs::{MatcherJob, ShellCommand},
};

#[action("Select theme")]
fn select_theme(editor: &mut Editor, id: ClientId) {
    let themes: Vec<String> = editor
        .themes
        .names()
        .into_iter()
        .map(String::from)
        .collect();
    let (win, _buf) = editor.win_buf_mut(id);

    let job = MatcherJob::builder(id)
        .options(Arc::new(themes))
        .handler(Prompt::matcher_result_handler)
        .build();

    win.prompt = Prompt::builder()
        .prompt("Select theme")
        .on_confirm(move |editor, id, input| match editor.themes.get(input) {
            Ok(t) => {
                let theme = t.clone();
                let (win, _buf) = editor.win_buf_mut(id);
                win.display_options_mut().theme = input.into();
                editor.send_to_client(id, ClientMessage::Theme(theme));
            }
            Err(_) => {
                let (win, _buf) = editor.win_buf_mut(id);
                win.warn_msg(&format!("No such theme '{}'", input));
            }
        })
        .build();
    win.focus = Focus::Prompt;

    editor.job_broker.request(job);
}

#[action("Command palette")]
fn command_palette(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);

    let job = MatcherJob::builder(id)
        .options(Arc::new(commands::command_palette()))
        .handler(commands::matcher_result_handler)
        .build();

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
    const PROMPT_MESSAGE: &str = "Open a file";
    let path = editor.working_dir().to_path_buf();
    let job = MatcherJob::builder(id)
        .options(FileOptionProvider::new(&path))
        .handler(Prompt::matcher_result_handler)
        .build();
    editor.job_broker.request_slot(id, PROMPT_MESSAGE, job);
    let (win, _buf) = editor.win_buf_mut(id);

    win.prompt = Prompt::builder()
        .prompt(PROMPT_MESSAGE)
        .on_confirm(move |editor, id, input| {
            let path = PathBuf::from(input);

            if let Err(e) = editor.open_file(id, &path) {
                let (win, _buf) = editor.win_buf_mut(id);
                win.warn_msg(&format!("Failed to open file {input}"))
            }
        })
        .build();
    win.focus = Focus::Prompt;
}

#[action("Close prompt")]
fn close(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.focus = Focus::Window;

    let slotname = win.prompt.message().to_string();
    editor.job_broker.stop_slot(id, &slotname);

    let (win, _buf) = editor.win_buf_mut(id);
    if let Some(on_abort) = win.prompt.on_abort() {
        let input = win.prompt.input_or_selected();
        (on_abort)(editor, id, &input)
    }
}

#[action("Confirm selection")]
fn confirm(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.focus = Focus::Window;

    let slotname = win.prompt.message().to_string();
    editor.job_broker.stop_slot(id, &slotname);

    let (win, _buf) = editor.win_buf_mut(id);
    if let Some(on_confirm) = win.prompt.on_confirm() {
        win.prompt.save_to_history();
        let input = win.prompt.input_or_selected();
        (on_confirm)(editor, id, &input)
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
        .simple()
        .on_confirm(move |editor, id, input| {
            let job = ShellCommand::new(id, input);
            editor.job_broker.request(job);
        })
        .build();
    win.focus = Focus::Prompt;
}

#[action("Goto a line")]
fn goto_line(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);

    win.prompt = Prompt::builder()
        .prompt("Line")
        .simple()
        .on_confirm(move |editor, id, input| {
            if let Ok(num) = input.parse::<usize>() {
                let (win, buf) = editor.win_buf_mut(id);
                win.goto_line(num, buf);
                hooks::run(editor, id, Hook::CursorMoved);
            }
        })
        .build();
    win.focus = Focus::Prompt;
}

#[action("Goto a percentage")]
fn goto_percentage(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);

    win.prompt = Prompt::builder()
        .prompt("Percentage")
        .simple()
        .on_confirm(move |editor, id, input| {
            if let Ok(mut num) = input.parse::<usize>() {
                num = min(100, num);
                let (win, buf) = editor.win_buf_mut(id);
                let offset = num * buf.len() / 100;
                win.goto_offset(offset, buf);
                hooks::run(editor, id, Hook::CursorMoved);
            }
        })
        .build();
    win.focus = Focus::Prompt;
}
