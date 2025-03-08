use sanedit_utils::either::Either;

use crate::editor::{
    windows::{Focus, Prompt},
    Editor,
};

use sanedit_server::ClientId;

use super::ActionResult;

#[action("Locations: Clear")]
fn clear_locations(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    win.locations.clear();
    ActionResult::Ok
}

#[action("Locations: Show")]
fn show_locations(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    win.locations.show = true;
    win.focus_to(Focus::Locations);
    ActionResult::Ok
}

#[action("Locations: Close")]
fn close_locations(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    win.locations.show = false;
    win.focus_to(Focus::Window);
    ActionResult::Ok
}

#[action("Locations: Focus")]
fn focus_locations(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    if win.locations.show {
        win.focus_to(Focus::Locations);
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
                if let Err(e) = editor.open_file(id, &path) {
                    let (win, _buf) = editor.win_buf_mut(id);
                    win.error_msg(&format!("Failed to open file: {e}"));
                    return ActionResult::Failed;
                }

                let (win, buf) = editor.win_buf_mut(id);
                win.goto_offset(offset, buf);
                win.focus_to(Focus::Window);
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

#[action("Locations: Keep entries with")]
fn keep_locations(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    win.prompt = Prompt::builder()
        .prompt("Keep locations")
        .simple()
        .on_confirm(move |editor, id, out| {
            let (win, _buf) = editor.win_buf_mut(id);
            let text = get!(out.text());
            win.locations.retain(|name| name.contains(text));
            win.focus_to(Focus::Locations);
        })
        .build();
    win.focus_to(Focus::Prompt);
    ActionResult::Ok
}

#[action("Locations: Reject entries with")]
fn reject_locations(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    win.prompt = Prompt::builder()
        .prompt("Reject locations")
        .simple()
        .on_confirm(move |editor, id, out| {
            let (win, _buf) = editor.win_buf_mut(id);
            let text = get!(out.text());
            win.locations.retain(|name| !name.contains(text));
            win.focus_to(Focus::Locations);
        })
        .build();
    win.focus_to(Focus::Prompt);
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
