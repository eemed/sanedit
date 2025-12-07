use std::{path::Path, sync::Arc};

use sanedit_core::Group;
use sanedit_utils::either::Either;

use crate::{actions::jobs::{FileOptionProvider, LocationsGlobAdd}, editor::{
    windows::{Focus, Prompt},
    Editor,
}};

use sanedit_server::ClientId;

use super::{window::focus, ActionResult};

#[action("Locations: Select first entry")]
fn loc_select_first(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    win.locations.select_first();
    ActionResult::Ok
}

#[action("Locations: Select last entry")]
fn loc_select_last(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    win.locations.select_last();
    ActionResult::Ok
}

#[action("Locations: Clear")]
fn clear_locations(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    win.locations.clear();
    ActionResult::Ok
}

#[action("Locations: Show")]
fn show_locations(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    win.locations.extra.show = true;
    focus(editor, id, Focus::Locations);
    ActionResult::Ok
}

#[action("Locations: Close")]
fn close_locations(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    win.locations.extra.show = false;
    focus(editor, id, Focus::Window);
    ActionResult::Ok
}

#[action("Locations: Focus")]
fn focus_locations(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    if win.locations.extra.show {
        focus(editor, id, Focus::Locations);
    }

    ActionResult::Ok
}

#[action("Locations: Next entry")]
fn next_loc_entry(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    win.locations.select_next();
    ActionResult::Ok
}

#[action("Locations: Previous entry")]
fn prev_loc_entry(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    win.locations.select_prev();
    ActionResult::Ok
}

#[action("Locations: Confirm entry")]
fn goto_loc_entry(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);

    if let Some(sel) = win.locations.selected_mut() {
        match sel {
            Either::Left(group) => {
                if group.is_expanded() {
                    group.collapse();
                } else {
                    group.expand();
                }
            }
            Either::Right(item) => {
                let hl_off = item.highlights().first().map_or(0, |r| r.start);
                let offset = item.absolute_offset().unwrap_or(0) + hl_off as u64;
                let parent = getf!(win.locations.parent_of_selected());
                let path = parent.path().to_path_buf();

                // Push current position to jumps
                let (win, buf) = editor.win_buf_mut(id);
                win.push_new_cursor_jump(buf);

                if let Err(e) = editor.open_file(id, &path) {
                    let (win, _buf) = editor.win_buf_mut(id);
                    win.error_msg(&format!("Failed to open file: {e}"));
                    return ActionResult::Failed;
                }

                let (win, buf) = editor.win_buf_mut(id);
                win.goto_offset(offset, buf);
                focus(editor, id, Focus::Window);
            }
        }
    }

    ActionResult::Ok
}

#[action("Locations: Goto parent")]
fn select_loc_parent(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    win.locations.select_parent();
    ActionResult::Ok
}

#[action("Locations: Expand / collapse toggle")]
fn toggle_all_expand_locs(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    let mut has_expanded = false;
    for group in win.locations.groups() {
        if group.is_expanded() {
            has_expanded = true;
            break;
        }
    }

    if has_expanded {
        win.locations.collapse_all();
    } else {
        win.locations.expand_all();
    }

    ActionResult::Ok
}

#[action("Locations: Keep files with")]
fn keep_locations(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    win.prompt = Prompt::builder()
        .prompt("Keep location files")
        .simple()
        .on_confirm(move |editor, id, out| {
            let (win, _buf) = editor.win_buf_mut(id);
            let text = getf!(out.text());
            let case_sensitive = text.chars().any(|ch| ch.is_uppercase());
            win.locations.retain(|name| {
                if case_sensitive {
                    name.contains(text)
                } else {
                    let lowercase_name = name.to_lowercase();
                    lowercase_name.contains(text)
                }
            });
            focus(editor, id, Focus::Locations);
            ActionResult::Ok
        })
        .build();
    focus(editor, id, Focus::Prompt);
    ActionResult::Ok
}

#[action("Locations: Reject files with")]
fn reject_locations(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    win.prompt = Prompt::builder()
        .prompt("Reject location files")
        .simple()
        .on_confirm(move |editor, id, out| {
            let (win, _buf) = editor.win_buf_mut(id);
            let text = getf!(out.text());
            let case_sensitive = text.chars().any(|ch| ch.is_uppercase());
            win.locations.retain(|name| {
                if case_sensitive {
                    !name.contains(text)
                } else {
                    let lowercase_name = name.to_lowercase();
                    !lowercase_name.contains(text)
                }
            });
            focus(editor, id, Focus::Locations);
            ActionResult::Ok
        })
        .build();
    focus(editor, id, Focus::Prompt);
    ActionResult::Ok
}

#[action("Locations: Open next item")]
fn goto_next_loc_item(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    if win.locations.select_next_item() {
        goto_loc_entry.execute(editor, id);
    }
    ActionResult::Ok
}

#[action("Locations: Open previous item")]
fn goto_prev_loc_item(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    if win.locations.select_prev_item() {
        goto_loc_entry.execute(editor, id);
    }
    ActionResult::Ok
}

#[action("Locations: Stop backing job")]
fn loc_stop_job(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    if let Some(job) = win.locations.extra.job.take() {
        editor.job_broker.stop(job);
    }
    ActionResult::Ok
}

#[action("Locations: Open next file")]
fn goto_next_loc_file(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    if !win.locations.select_next_group() {
        return ActionResult::Skipped;
    }

    if let Some(path) = win
        .locations
        .selected()
        .and_then(Either::take_left)
        .map(Group::path)
        .map(Path::to_path_buf)
    {
        return editor.open_file(id, &path).into();
    }

    ActionResult::Ok
}

#[action("Locations: Open previous file")]
fn goto_prev_loc_file(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    if !win.locations.select_prev_group() {
        return ActionResult::Skipped;
    }

    if let Some(path) = win
        .locations
        .selected()
        .and_then(Either::take_left)
        .map(Group::path)
        .map(Path::to_path_buf)
    {
        return editor.open_file(id, &path).into();
    }

    ActionResult::Ok
}

#[action("Locations: Add files to locations")]
fn loc_add_groups(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    win.prompt = Prompt::builder()
        .prompt("Add location files (glob)")
        .simple()
        .on_confirm(move |editor, id, out| {
            let pattern = getf!(out.text());
            let ignore = editor.ignore.clone();
            let wd = editor.working_dir().to_path_buf();
            let opts = FileOptionProvider::new(&wd, ignore, editor.config.editor.git_ignore);
            let Ok(job) = LocationsGlobAdd::new(id, pattern, Arc::new(opts)) else { return ActionResult::Failed; };
            editor.job_broker.request(job);
            ActionResult::Ok
        })
        .build();
    focus(editor, id, Focus::Prompt);
    ActionResult::Ok
}
