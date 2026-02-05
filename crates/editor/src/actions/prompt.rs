mod commands;

use std::{
    cmp::min,
    path::{Path, PathBuf},
    sync::Arc,
    time::Instant,
};

use sanedit_buffer::PieceTreeSlice;
use sanedit_messages::{key::try_parse_keyevents, ClientMessage};
use sanedit_utils::idmap::AsID;

use crate::{
    actions::{
        cursors::jump_to_ref,
        hooks::run,
        jobs::{FileOptionProvider, Grep, MatchStrategy},
        window::focus,
    },
    common::{human_readable_duration, is_yes, Choice},
    editor::{
        buffers::BufferId,
        hooks::Hook,
        keymap::KeymapResult,
        windows::{Focus, HistoryKind, Prompt},
        Editor, Map,
    },
};

use sanedit_server::{ClientId, JobId};

use super::{
    find_by_description, hooks,
    jobs::{DirectoryOptionProvider, MatcherJob},
    text::save,
    window::{focus_with_mode, mode_normal},
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
        .loads_options()
        .on_confirm(move |editor, id, out| {
            let text = getf!(out.text());
            match editor.themes.get(text) {
                Ok(t) => {
                    let theme = t.clone();
                    let (win, _buf) = editor.win_buf_mut(id);
                    win.config.theme = text.into();
                    editor.send_to_client(id, ClientMessage::Theme(theme).into());
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
        .handler(Prompt::matcher_result_handler)
        .build();

    win.prompt = Prompt::builder()
        .prompt("Commands")
        .loads_options()
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
    let ignore = editor.ignore.clone();
    let wd = editor.working_dir().to_path_buf();
    let job = MatcherJob::builder(id)
        .options(FileOptionProvider::new(
            &wd,
            ignore,
            editor.config.editor.git_ignore,
        ))
        .handler(Prompt::open_file_handler)
        .build();
    editor.job_broker.request_slot(id, PROMPT_MESSAGE, job);
    let (win, _buf) = editor.win_buf_mut(id);

    win.prompt = Prompt::builder()
        .prompt(PROMPT_MESSAGE)
        .loads_options()
        .on_confirm(move |editor, id, out| {
            let path = getf!(out.path_selection());

            // Record a jump here before opening a new buffer
            let (win, buf) = editor.win_buf_mut(id);
            win.push_new_cursor_jump(buf);

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
        .loads_options()
        .on_confirm(move |editor, id, out| {
            let num = getf!(out.number());
            let bid = BufferId::from(num);
            let (win, _buf) = editor.win_buf_mut(id);
            let ok_buf = win.buffer_id() != bid && editor.buffers.get(bid).is_some();
            if !ok_buf {
                return ActionResult::Failed;
            }
            let (win, buf) = editor.win_buf_mut(id);
            win.push_new_cursor_jump(buf);
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
                win.jump_to_offset(offset, buf);
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
    prompt_change_dir(editor, id, &wd, true);
    ActionResult::Ok
}

pub(crate) fn get_directory_searcher_term(display: &str) -> &str {
    let mut len = 0;

    for ch in display.chars().rev() {
        if ch == std::path::MAIN_SEPARATOR {
            break;
        }

        len += ch.len_utf8();
    }

    &display[display.len() - len..]
}

fn prompt_change_dir(editor: &mut Editor, id: ClientId, input: &Path, is_dir: bool) {
    const JOB_NAME: &str = "directory-select";
    let mut path = PathBuf::from(input);
    while !path.is_dir() {
        match path.parent() {
            Some(parent) => {
                path = parent.into();
            }
            None => break,
        }
    }
    let ignore = editor.ignore.clone();
    let (win, _buf) = editor.win_buf_mut(id);
    let mut display = input.to_string_lossy().to_string();
    let has_end = display.ends_with(std::path::MAIN_SEPARATOR);
    if !has_end && is_dir {
        display.push(std::path::MAIN_SEPARATOR);
    };
    let search = get_directory_searcher_term(&display);
    let job = MatcherJob::builder(id)
        .options(DirectoryOptionProvider::new_non_recursive(&path, ignore))
        .handler(Prompt::matcher_result_handler_directory_selector)
        .strategy(MatchStrategy::Prefix)
        .search(search.to_string())
        .build();
    win.prompt = Prompt::builder()
        .prompt("Select directory")
        .loads_options()
        .simple()
        .input(&display)
        .on_input(move |e, id, input| {
            let npath = PathBuf::from(input);
            if npath.strip_prefix(&path).is_err() {
                prompt_change_dir(e, id, &npath, false);
            }
        })
        .on_confirm(|e, id, out| {
            let path = getf!(out.path());
            if !out.is_selection() {
                if let Err(err) = e.change_working_dir(&path) {
                    let (win, _buf) = e.win_buf_mut(id);
                    win.warn_msg(&err.to_string());
                    return ActionResult::Failed;
                }

                return ActionResult::Ok;
            }

            prompt_change_dir(e, id, &path, true);
            ActionResult::Ok
        })
        .build();
    focus(editor, id, Focus::Prompt);
    editor.job_broker.request_slot(id, JOB_NAME, job);
}

#[action("Grep")]
fn grep(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    win.prompt = Prompt::builder()
        .prompt("Grep")
        .history(HistoryKind::Grep)
        .simple()
        .on_confirm(move |e, id, out| {
            let patt = getf!(out.text());
            let ignore = e.ignore.clone();
            let wd = e.working_dir();
            let buffers: Map<PathBuf, PieceTreeSlice> = {
                let mut map = Map::default();

                for (_, buf) in e.buffers().iter() {
                    // If not modified we grep from disk
                    if !buf.is_modified() {
                        continue;
                    }

                    if let Some(path) = buf.path() {
                        map.insert(path.to_path_buf(), buf.slice(..));
                    }
                }

                map
            };
            let job = Grep::new(patt, wd, ignore, buffers, id, e.config.editor.git_ignore);
            let job_name = format!("Grep '{patt}'");
            e.job_broker.request_slot(id, &job_name, job);
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
                    editor.open_buffer(id, bid);
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
            let buf = getf!(editor.buffers().get(bid));
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
        let choice = Choice::from_numbered_text(items.len() + 1, text);
        items.push(choice);
        cursors.push(cursor.clone());
        item = win.cursor_jumps.prev_of_ref(&cursor);
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
        .loads_options()
        .on_confirm(move |editor, id, out| {
            let index = getf!(out.number()) - 1;
            let cursor = cursors[index].clone();
            jump_to_ref(editor, id, cursor)
        })
        .build();
    focus(editor, id, Focus::Prompt);
    ActionResult::Ok
}


#[action("Buffer: Show undopoints")]
fn buffer_undopoints(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (_win, buf) = win_buf!(editor, id);
    let now = Instant::now();
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
            let since = now.duration_since(snapshot.timestamp);
            let ts = human_readable_duration(since);
            let is_current = current.map(|c| c == snapshot.id).unwrap_or(false);
            let text = if is_current {
                format!("> Snapshot {ts}")
            } else {
                format!("Snapshot {ts}")
            };
            // Reverse order using numbering
            Choice::from_numbered_text(nodes_len - snapshot.id, text)
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
        .loads_options()
        .on_confirm(move |editor, id, out| {
            let snapshot = nodes_len - getf!(out.number());
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

#[action("Editor: Kill jobs")]
fn kill_jobs(editor: &mut Editor, id: ClientId) -> ActionResult {
    const PROMPT_MESSAGE: &str = "Kill a job";
    let jobs = editor.job_broker.jobs();
    let options: Vec<Arc<Choice>> = jobs
        .iter()
        .map(|(jid, slot)| {
            let text = match slot {
                Some((id, name)) => format!("{name} Client: {}", id.as_usize()),
                None => "No description".into(),
            };
            Choice::from_numbered_text(jid.as_usize(), text)
        })
        .collect();
    let job = MatcherJob::builder(id)
        .options(Arc::new(options))
        .handler(Prompt::matcher_result_handler)
        .build();
    editor.job_broker.request_slot(id, PROMPT_MESSAGE, job);
    let (win, _buf) = editor.win_buf_mut(id);
    win.prompt = Prompt::builder()
        .prompt(PROMPT_MESSAGE)
        .loads_options()
        .on_confirm(move |editor, _id, out| {
            let n = getf!(out.number());
            let jid = JobId::from(n);
            match jobs.get(&jid) {
                Some(Some((id, name))) => editor.job_broker.stop_slot(*id, name),
                _ => editor.job_broker.stop(jid),
            }
            ActionResult::Ok
        })
        .build();
    focus(editor, id, Focus::Prompt);
    ActionResult::Ok
}

#[action("Editor: Show key mappings")]
fn show_keymaps(editor: &mut Editor, id: ClientId) -> ActionResult {
    const PROMPT_MESSAGE: &str = "Maps";

    let (win, _buf) = editor.win_buf(id);
    let key = win.layer();
    let maps: Arc<Vec<Arc<Choice>>> = Arc::new(
        editor
            .keymaps
            .list(&key)
            .into_iter()
            .map(|(map, name)| Choice::from_text_with_description(map, name))
            .collect(),
    );
    let job = MatcherJob::builder(id)
        .options(maps.clone())
        .handler(Prompt::matcher_result_handler)
        .build();
    editor.job_broker.request_slot(id, PROMPT_MESSAGE, job);
    let (win, _buf) = editor.win_buf_mut(id);
    win.prompt = Prompt::builder()
        .prompt(PROMPT_MESSAGE)
        .loads_options()
        .on_abort(move |editor, id, _out| {
            if key.focus != Focus::Prompt {
                focus_with_mode(editor, id, key.focus, key.mode);
            }
        })
        .on_confirm(move |editor, id, out| {
            if key.focus != Focus::Prompt {
                focus_with_mode(editor, id, key.focus, key.mode);

                let text = getf!(out.text());
                let events = getf!(try_parse_keyevents(text).ok());
                if let KeymapResult::Matched(action) = editor.keymaps.get(&key, &events) {
                    action.execute(editor, id);
                }
            }
            ActionResult::Ok
        })
        .build();
    focus(editor, id, Focus::Prompt);
    ActionResult::Ok
}
