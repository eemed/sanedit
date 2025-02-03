mod commands;

use std::{cmp::min, path::PathBuf, sync::Arc};

use rustc_hash::FxHashMap;
use sanedit_buffer::PieceTreeView;
use sanedit_messages::ClientMessage;
use sanedit_utils::idmap::{AsID, ID};

use crate::{
    actions::jobs::{FileOptionProvider, MatchedOptions},
    common::{is_yes, matcher::Choice},
    editor::{
        buffers::BufferId,
        hooks::Hook,
        windows::{Focus, HistoryKind, Prompt},
        Editor,
    },
};

use sanedit_server::ClientId;

use super::{
    find_by_description, hooks,
    jobs::{Grep, MatcherJob, MatcherMessage},
    shell,
    text::save,
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
        .on_confirm(move |editor, id, out| {
            let text = get!(out.text());
            match editor.themes.get(text) {
                Ok(t) => {
                    let theme = t.clone();
                    let (win, _buf) = editor.win_buf_mut(id);
                    win.config.theme = text.into();
                    editor.send_to_client(id, ClientMessage::Theme(theme));
                }
                Err(_) => {
                    let (win, _buf) = editor.win_buf_mut(id);
                    win.warn_msg(&format!("No such theme '{}'", text));
                }
            }
        })
        .build();

    editor.job_broker.request(job);
}

#[action("Command palette")]
fn command_palette(editor: &mut Editor, id: ClientId) {
    let opts = Arc::new(commands::command_palette(editor, id));

    let (win, _buf) = editor.win_buf_mut(id);
    let job = MatcherJob::builder(id)
        .options(opts)
        .handler(commands::matcher_result_handler)
        .build();

    win.prompt = Prompt::builder()
        .prompt("Palette")
        .on_confirm(move |editor, id, out| {
            let desc = get!(out.text());
            if let Some(action) = find_by_description(desc) {
                action.execute(editor, id);
                return;
            }

            log::error!("No action with name {desc}");
        })
        .build();

    editor.job_broker.request(job);
}

#[action("Open a file")]
fn open_file(editor: &mut Editor, id: ClientId) {
    const PROMPT_MESSAGE: &str = "Open a file";
    let ignore = editor.config.editor.ignore_directories();
    let wd = editor.working_dir().to_path_buf();
    let job = MatcherJob::builder(id)
        .options(FileOptionProvider::new(&wd, &ignore))
        .handler(open_file_handler)
        .build();
    editor.job_broker.request_slot(id, PROMPT_MESSAGE, job);
    let (win, _buf) = editor.win_buf_mut(id);

    win.prompt = Prompt::builder()
        .prompt(PROMPT_MESSAGE)
        .on_confirm(move |editor, id, out| {
            let path = get!(out.path());

            match editor.open_file(id, &path) {
                Ok(()) => {
                    editor.caches.files.insert(path);
                }
                Err(e) => {
                    let (win, _buf) = editor.win_buf_mut(id);
                    win.warn_msg(&format!("Failed to open file {path:?}: {e}"));
                }
            }
        })
        .build();
    win.focus_to(Focus::Prompt);
}

fn open_file_handler(editor: &mut Editor, id: ClientId, msg: MatcherMessage) {
    use MatcherMessage::*;

    let draw = editor.draw_state(id);
    draw.no_redraw_window();

    let (win, _buf) = win_buf!(editor, id);
    match msg {
        Init(sender) => {
            win.prompt.set_on_input(move |_editor, _id, input| {
                let _ = sender.blocking_send(input.to_string());
            });
            win.prompt.clear_choices();
        }
        Progress(opts) => {
            if let MatchedOptions::Options { matched, clear_old } = opts {
                if clear_old {
                    win.prompt.clear_choices();
                }

                let no_input = matched
                    .get(0)
                    .map(|choice| choice.matches().is_empty())
                    .unwrap_or(false);

                let mut rescored = matched;
                if no_input {
                    // If no input is matched, sort results using LRU

                    let cache = &mut editor.caches.files;
                    let lru = cache.to_map();
                    let max = lru.len();
                    for mut mat in std::mem::take(&mut rescored).into_iter() {
                        let path = match mat.choice() {
                            Choice::Path { path, .. } => path,
                            _ => unreachable!(),
                        };
                        if let Some(score) = lru.get(path.as_path()) {
                            mat.rescore(*score as u32);
                        } else {
                            mat.rescore(mat.score() + max as u32);
                        }
                        rescored.push(mat);
                    }
                }

                win.focus_to(Focus::Prompt);
                let (win, _buf) = editor.win_buf_mut(id);
                win.prompt.add_choices(rescored);
            }
        }
    }
}

#[action("Open a buffer")]
fn open_buffer(editor: &mut Editor, id: ClientId) {
    const PROMPT_MESSAGE: &str = "Open a buffer";
    let buffers: Vec<String> = editor
        .buffers()
        .iter()
        .map(|(bid, buf)| format!("{}: {}", bid.id(), buf.name()))
        .collect();
    let job = MatcherJob::builder(id)
        .options(Arc::new(buffers))
        .handler(Prompt::matcher_result_handler)
        .build();
    editor.job_broker.request_slot(id, PROMPT_MESSAGE, job);
    let (win, _buf) = editor.win_buf_mut(id);

    win.prompt = Prompt::builder()
        .prompt(PROMPT_MESSAGE)
        .on_confirm(move |editor, id, out| {
            // TODO maybe own type in prompt?
            let text = get!(out.text());
            if let Some(bid) = text.split(':').next() {
                if let Ok(bid) = bid.parse::<ID>() {
                    let bid = BufferId::from(bid);
                    editor.open_buffer(id, bid);
                }
            }
        })
        .build();
    win.focus_to(Focus::Prompt);
}

#[action("Close prompt")]
fn prompt_close(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.focus_to(Focus::Window);

    let slotname = win.prompt.message().to_string();
    editor.job_broker.stop_slot(id, &slotname);

    let (win, _buf) = editor.win_buf_mut(id);
    if let Some(on_abort) = win.prompt.on_abort() {
        let input = win.prompt.input_or_selected();
        (on_abort)(editor, id, input)
    }
}

#[action("Confirm selection")]
fn prompt_confirm(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.focus_to(Focus::Window);

    let slotname = win.prompt.message().to_string();
    editor.job_broker.stop_slot(id, &slotname);

    let (win, _buf) = editor.win_buf_mut(id);
    if let Some(on_confirm) = win.prompt.on_confirm() {
        let out = win.prompt.input_or_selected();
        if let Some(text) = out.text() {
            if let Some(kind) = win.prompt.history() {
                let history = editor.histories.entry(kind).or_default();
                history.push(text);
            }
        }
        (on_confirm)(editor, id, out)
    }
}

#[action("Move cursor one character right")]
fn prompt_next_grapheme(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.prompt.next_grapheme();
}

#[action("Move cursor one character left")]
fn prompt_prev_grapheme(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.prompt.prev_grapheme();
}

#[action("Delete a character before cursor")]
pub(crate) fn prompt_remove_grapheme_before_cursor(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.prompt.remove_grapheme_before_cursor();

    if let Some(on_input) = win.prompt.on_input() {
        let input = win.prompt.input().to_string();
        (on_input)(editor, id, &input)
    }
}

#[action("Select the next completion item")]
fn prompt_next_completion(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.prompt.next_completion();
}

#[action("Select the previous completion item")]
fn prompt_prev_completion(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.prompt.prev_completion();
}

#[action("Select the next entry from history")]
fn prompt_history_next(editor: &mut Editor, id: ClientId) {
    editor.prompt_history_next(id);
}

#[action("Select the previous entry from history")]
fn prompt_history_prev(editor: &mut Editor, id: ClientId) {
    editor.prompt_history_prev(id);
}

#[action("Run a shell command")]
fn shell_command(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);

    win.prompt = Prompt::builder()
        .prompt("Shell")
        .history(HistoryKind::Command)
        .simple()
        .on_confirm(shell::execute_prompt)
        .build();
    win.focus_to(Focus::Prompt);
}

#[action("Goto a line")]
fn goto_line(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);

    win.prompt = Prompt::builder()
        .prompt("Line")
        .simple()
        .on_confirm(move |editor, id, out| {
            let text = get!(out.text());
            if let Ok(num) = text.parse::<u64>() {
                let (win, buf) = editor.win_buf_mut(id);
                win.goto_line(num, buf);
                hooks::run(editor, id, Hook::CursorMoved);
            }
        })
        .build();
    win.focus_to(Focus::Prompt);
}

#[action("Goto a percentage")]
fn goto_percentage(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);

    win.prompt = Prompt::builder()
        .prompt("Percentage")
        .simple()
        .on_confirm(move |editor, id, out| {
            let text = get!(out.text());
            if let Ok(mut num) = text.parse::<u64>() {
                num = min(100, num);
                let (win, buf) = editor.win_buf_mut(id);
                let offset = num * buf.len() / 100;
                win.goto_offset(offset, buf);
                hooks::run(editor, id, Hook::CursorMoved);
            }
        })
        .build();
    win.focus_to(Focus::Prompt);
}

#[action("Change working directory")]
fn change_working_dir(editor: &mut Editor, id: ClientId) {
    let wd = editor.working_dir().to_path_buf();
    let (win, _buf) = editor.win_buf_mut(id);
    win.prompt = Prompt::builder()
        .prompt("Working directory")
        .simple()
        .input(&wd.to_string_lossy())
        .on_confirm(move |e, id, out| {
            let path = get!(out.path());
            if let Err(err) = e.change_working_dir(&path) {
                let (win, _buf) = e.win_buf_mut(id);
                win.warn_msg(&err.to_string());
            }
        })
        .build();
    win.focus_to(Focus::Prompt);
}

#[action("Grep")]
fn grep(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.prompt = Prompt::builder()
        .prompt("Grep")
        .simple()
        .on_confirm(move |e, id, out| {
            const GREP_JOB: &str = "grep";
            let patt = get!(out.text());
            let ignore = e.config.editor.ignore_directories();
            let wd = e.working_dir();
            let buffers: FxHashMap<PathBuf, PieceTreeView> = {
                let mut map = FxHashMap::default();

                for (_, buf) in e.buffers().iter() {
                    // If not modified we let ripgrep grep from disk
                    if !buf.is_modified() {
                        continue;
                    }

                    if let Some(path) = buf.path() {
                        map.insert(path.to_path_buf(), buf.ro_view());
                    }
                }

                map
            };
            let job = Grep::new(patt, wd, &ignore, buffers, id);
            e.job_broker.request_slot(id, GREP_JOB, job);
        })
        .build();
    win.focus_to(Focus::Prompt);
}

// TODO use
#[allow(dead_code)]
/// Prompt whether buffer changes should be changed or not
pub(crate) fn close_modified_buffer<F: Fn(&mut Editor, ClientId) + 'static>(
    editor: &mut Editor,
    id: ClientId,
    on_confirm: F,
) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.prompt = Prompt::builder()
        .prompt("Closing a modified buffer. Save changes? (Y/n)")
        .simple()
        .on_confirm(move |e, id, out| {
            let ans = get!(out.text());
            let yes = ans.is_empty() || is_yes(ans);
            if yes {
                save.execute(e, id);
            }

            (on_confirm)(e, id);
        })
        .build();
    win.focus_to(Focus::Prompt);
}
