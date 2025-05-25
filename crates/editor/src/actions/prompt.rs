mod commands;

use std::{
    cmp::min,
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

use chrono::{DateTime, Local, TimeDelta};
use rustc_hash::FxHashMap;
use sanedit_buffer::PieceTreeView;
use sanedit_messages::ClientMessage;
use sanedit_utils::idmap::AsID;

use crate::{
    actions::{
        cursors::jump_to_ref,
        hooks::run,
        jobs::{FileOptionProvider, Grep, MatchedOptions},
        window::focus,
    },
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
    jobs::{MatcherJob, MatcherMessage},
    shell,
    text::save,
    window::mode_normal,
    ActionResult,
};

#[action("Editor: Select theme")]
fn select_theme(editor: &mut Editor, id: ClientId) -> ActionResult {
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
            let text = getf!(out.text());
            match editor.themes.get(text) {
                Ok(t) => {
                    let theme = t.clone();
                    let (win, _buf) = editor.win_buf_mut(id);
                    win.config.theme = text.into();
                    editor.send_to_client(id, ClientMessage::Theme(theme));
                    ActionResult::Ok
                }
                Err(_) => {
                    let (win, _buf) = editor.win_buf_mut(id);
                    win.warn_msg(&format!("No such theme '{}'", text));
                    ActionResult::Failed
                }
            }
        })
        .build();

    editor.job_broker.request(job);
    ActionResult::Ok
}

#[action("Editor: Command palette")]
fn command_palette(editor: &mut Editor, id: ClientId) -> ActionResult {
    let opts = Arc::new(commands::command_palette(editor, id));

    let (win, _buf) = editor.win_buf_mut(id);
    let job = MatcherJob::builder(id)
        .options(opts)
        .handler(commands::matcher_result_handler)
        .build();

    win.prompt = Prompt::builder()
        .prompt("Commands")
        .on_confirm(move |editor, id, out| {
            let desc = getf!(out.text());
            if let Some(action) = find_by_description(desc) {
                action.execute(editor, id);
                return ActionResult::Ok;
            }

            log::error!("No action with name {desc}");
            ActionResult::Failed
        })
        .build();

    editor.job_broker.request(job);
    ActionResult::Ok
}

#[action("Editor: Open file")]
fn open_file(editor: &mut Editor, id: ClientId) -> ActionResult {
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
            let path = getf!(out.path());

            match editor.open_file(id, &path) {
                Ok(()) => {
                    editor.caches.files.insert(path);
                    ActionResult::Ok
                }
                Err(e) => {
                    let (win, _buf) = editor.win_buf_mut(id);
                    win.warn_msg(&format!("Failed to open file {path:?}: {e}"));
                    ActionResult::Failed
                }
            }
        })
        .build();
    focus(editor, id, Focus::Prompt);
    ActionResult::Ok
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

                let (win, _buf) = editor.win_buf_mut(id);
                win.prompt.add_choices(rescored);

                focus(editor, id, Focus::Prompt);
            }
        }
    }
}

#[action("Editor: Open buffer")]
fn open_buffer(editor: &mut Editor, id: ClientId) -> ActionResult {
    const PROMPT_MESSAGE: &str = "Open a buffer";
    let wd = editor.working_dir();
    let buffers: Vec<Arc<Choice>> = editor
        .buffers()
        .iter()
        .map(|(bid, buf)| {
            let path = buf
                .path()
                .map(|path| {
                    let path = path.strip_prefix(wd).unwrap_or(path);
                    path.display().to_string().into()
                })
                .unwrap_or(buf.name());
            let modified = if buf.is_modified() { " *" } else { "" };
            let text = format!("{}{}", path, modified);
            Choice::from_numbered_text(bid.id(), text)
        })
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
            let num = getf!(out.number());
            let bid = BufferId::from(num);
            editor.open_buffer(id, bid);
            ActionResult::Ok
        })
        .build();
    focus(editor, id, Focus::Prompt);
    ActionResult::Ok
}

#[action("Prompt: Close")]
fn prompt_close(editor: &mut Editor, id: ClientId) -> ActionResult {
    mode_normal(editor, id);

    let (win, _buf) = editor.win_buf_mut(id);

    let slotname = win.prompt.message().to_string();
    editor.job_broker.stop_slot(id, &slotname);

    let (win, _buf) = editor.win_buf_mut(id);
    if let Some(on_abort) = win.prompt.on_abort() {
        let input = win.prompt.input_or_selected();
        (on_abort)(editor, id, input)
    }
    ActionResult::Ok
}

#[action("Prompt: Confirm")]
fn prompt_confirm(editor: &mut Editor, id: ClientId) -> ActionResult {
    mode_normal(editor, id);
    let (win, _buf) = editor.win_buf_mut(id);

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
        return (on_confirm)(editor, id, out);
    }
    ActionResult::Ok
}

#[action("Prompt: Move cursor right")]
fn prompt_next_grapheme(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    win.prompt.next_grapheme();
    ActionResult::Ok
}

#[action("Prompt: Move cursor left")]
fn prompt_prev_grapheme(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    win.prompt.prev_grapheme();
    ActionResult::Ok
}

#[action("Prompt: Delete grapheme before cursor")]
pub(crate) fn prompt_remove_grapheme_before_cursor(
    editor: &mut Editor,
    id: ClientId,
) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    win.prompt.remove_grapheme_before_cursor();

    if let Some(on_input) = win.prompt.on_input() {
        let input = win.prompt.input().to_string();
        (on_input)(editor, id, &input)
    }
    ActionResult::Ok
}

#[action("Prompt: Select next completion item")]
fn prompt_next_completion(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    win.prompt.next_completion();
    ActionResult::Ok
}

#[action("Prompt: Select previous completion item")]
fn prompt_prev_completion(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    win.prompt.prev_completion();
    ActionResult::Ok
}

#[action("Prompt: Select next history entry")]
fn prompt_history_next(editor: &mut Editor, id: ClientId) -> ActionResult {
    editor.prompt_history_next(id);
    ActionResult::Ok
}

#[action("Prompt: Select previous history entry")]
fn prompt_history_prev(editor: &mut Editor, id: ClientId) -> ActionResult {
    editor.prompt_history_prev(id);
    ActionResult::Ok
}

#[action("Editor: Run a shell command")]
fn shell_command(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);

    win.prompt = Prompt::builder()
        .prompt("Shell")
        .history(HistoryKind::Command)
        .simple()
        .on_confirm(shell::execute_prompt)
        .build();
    focus(editor, id, Focus::Prompt);
    ActionResult::Ok
}

#[action("Cursors: Goto line number")]
fn goto_line(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);

    win.prompt = Prompt::builder()
        .prompt("Line")
        .simple()
        .on_confirm(move |editor, id, out| {
            let text = getf!(out.text());
            if let Ok(num) = text.parse::<u64>() {
                let (win, buf) = editor.win_buf_mut(id);
                win.goto_line(num, buf);
                hooks::run(editor, id, Hook::CursorMoved);
                return ActionResult::Ok;
            }

            ActionResult::Failed
        })
        .build();
    focus(editor, id, Focus::Prompt);
    ActionResult::Ok
}

#[action("Cursors: Goto percentage")]
fn goto_percentage(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);

    win.prompt = Prompt::builder()
        .prompt("Percentage")
        .simple()
        .on_confirm(move |editor, id, out| {
            let text = getf!(out.text());
            if let Ok(mut num) = text.parse::<u64>() {
                num = min(100, num);
                let (win, buf) = editor.win_buf_mut(id);
                let offset = num * buf.len() / 100;
                win.goto_offset(offset, buf);
                hooks::run(editor, id, Hook::CursorMoved);

                return ActionResult::Ok;
            }

            ActionResult::Failed
        })
        .build();
    focus(editor, id, Focus::Prompt);
    ActionResult::Ok
}

#[action("Editor: Change working directory")]
fn change_working_dir(editor: &mut Editor, id: ClientId) -> ActionResult {
    let wd = editor.working_dir().to_path_buf();
    let (win, _buf) = editor.win_buf_mut(id);
    win.prompt = Prompt::builder()
        .prompt("Working directory")
        .simple()
        .input(&wd.to_string_lossy())
        .on_confirm(move |e, id, out| {
            let path = getf!(out.path());
            if let Err(err) = e.change_working_dir(&path) {
                let (win, _buf) = e.win_buf_mut(id);
                win.warn_msg(&err.to_string());
                return ActionResult::Failed;
            }

            ActionResult::Ok
        })
        .build();
    focus(editor, id, Focus::Prompt);
    ActionResult::Ok
}

#[action("Grep")]
fn grep(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    win.prompt = Prompt::builder()
        .prompt("Grep")
        .history(HistoryKind::Grep)
        .simple()
        .on_confirm(move |e, id, out| {
            const GREP_JOB: &str = "grep";
            let patt = getf!(out.text());
            let ignore = e.config.editor.ignore_directories();
            let wd = e.working_dir();
            let buffers: FxHashMap<PathBuf, PieceTreeView> = {
                let mut map = FxHashMap::default();

                for (_, buf) in e.buffers().iter() {
                    // If not modified we grep from disk
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
            ActionResult::Ok
        })
        .build();
    focus(editor, id, Focus::Prompt);
    ActionResult::Ok
}

/// Prompt whether buffer changes should be changed or not
pub(crate) fn unsaved_changes<F: Fn(&mut Editor, ClientId) -> ActionResult + 'static>(
    editor: &mut Editor,
    id: ClientId,
    on_confirm: F,
) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.prompt = Prompt::builder()
        .prompt("Save all unsaved changes? (Y/n)")
        .simple()
        .on_confirm(move |editor, id, out| {
            let ans = getf!(out.text());
            let yes = ans.is_empty() || is_yes(ans);
            if yes {
                let unsaved: Vec<BufferId> = editor
                    .buffers
                    .iter()
                    .filter(|(_, buf)| buf.is_modified())
                    .map(|(bid, _)| bid)
                    .collect();
                for bid in unsaved {
                    let (win, _buf) = win_buf!(editor, id);
                    win.open_buffer(bid);
                    save.execute(editor, id);
                }
            }

            (on_confirm)(editor, id)
        })
        .build();
    focus(editor, id, Focus::Prompt);
}

#[action("Window: Show cursor jumps")]
fn prompt_jump(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf(id);
    let current = win.cursor_jumps.current().map(|(cursor, _)| cursor);
    let mut items: Vec<Arc<Choice>> = vec![];
    let mut cursors = vec![];
    let mut item = win.cursor_jumps.last();

    while let Some((cursor, group)) = item {
        let bid = group.buffer_id();
        let text = {
            let positions = group
                .jumps()
                .iter()
                .map(|jump| jump.start().original_position().to_string())
                .collect::<Vec<String>>()
                .join(", ");
            let buf = editor.buffers().get(bid).unwrap();
            let wd = editor.working_dir();
            let name = match buf.path() {
                Some(path) => path.strip_prefix(wd).unwrap_or(path).to_string_lossy(),
                None => buf.name(),
            };
            let is_current = current
                .as_ref()
                .map(|current| current == &cursor)
                .unwrap_or(false);
            let current_indicator = if is_current { "> " } else { "" };
            format!("{}{} @ {}", current_indicator, name, positions)
        };
        let choice = Choice::from_numbered_text(items.len() as u32 + 1, text);
        items.push(choice);
        cursors.push(cursor.clone());
        item = win.cursor_jumps.prev(&cursor);
    }

    let items = Arc::new(items);
    const PROMPT_MESSAGE: &str = "Goto a jump";
    let job = MatcherJob::builder(id)
        .options(items.clone())
        .handler(Prompt::matcher_result_handler)
        .build();
    editor.job_broker.request_slot(id, PROMPT_MESSAGE, job);

    let (win, _buf) = editor.win_buf_mut(id);
    win.prompt = Prompt::builder()
        .prompt(PROMPT_MESSAGE)
        .on_confirm(move |editor, id, out| {
            let index = getf!(out.number()) as usize - 1;
            let cursor = cursors[index].clone();
            jump_to_ref(editor, id, cursor)
        })
        .build();
    focus(editor, id, Focus::Prompt);
    ActionResult::Ok
}

fn timestamp_to_string(local: &DateTime<Local>, since: Duration) -> String {
    let Ok(delta) = TimeDelta::from_std(since) else {
        return "<no-time>".into();
    };

    let Some(time) = local.checked_sub_signed(delta) else {
        return "<no-time>".into();
    };

    time.format("%H:%M:%S").to_string()
}

#[action("Buffer: Show snapshots")]
fn buffer_snapshots(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (_win, buf) = win_buf!(editor, id);
    let now = Instant::now();
    let local = Local::now();
    let on_snapshot = buf.on_undopoint();
    let current = if on_snapshot {
        buf.snapshots().current()
    } else {
        None
    };

    let nodes = buf.snapshots().nodes();
    let nodes_len = nodes.len();
    let snapshots: Vec<Arc<Choice>> = nodes
        .iter()
        .map(|snapshot| {
            let since = now.duration_since(snapshot.timestamp.clone());
            let ts = timestamp_to_string(&local, since);
            let is_current = current.map(|c| c == snapshot.id).unwrap_or(false);
            let text = if is_current {
                format!("> Snapshot at {ts}")
            } else {
                format!("Snapshot at {ts}")
            };
            // Reverse order using numbering
            Choice::from_numbered_text((nodes_len - snapshot.id) as u32, text)
        })
        .collect();

    let items = Arc::new(snapshots);
    const PROMPT_MESSAGE: &str = "Goto a undopoint";
    let job = MatcherJob::builder(id)
        .options(items.clone())
        .handler(Prompt::matcher_result_handler)
        .build();
    editor.job_broker.request_slot(id, PROMPT_MESSAGE, job);

    let (win, _buf) = editor.win_buf_mut(id);
    win.prompt = Prompt::builder()
        .prompt(PROMPT_MESSAGE)
        .on_confirm(move |editor, id, out| {
            let snapshot = nodes_len - getf!(out.number()) as usize;
            let (win, buf) = win_buf!(editor, id);
            if win.undo_jump(buf, snapshot).is_ok() {
                let hook = Hook::BufChanged(buf.id);
                run(editor, id, hook);
                run(editor, id, Hook::CursorMoved);

                return ActionResult::Ok;
            }

            ActionResult::Failed
        })
        .build();
    focus(editor, id, Focus::Prompt);
    ActionResult::Ok
}
