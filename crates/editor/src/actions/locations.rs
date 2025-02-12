use sanedit_utils::either::Either;

use crate::editor::{
    windows::{Focus, Prompt},
    Editor,
};

use sanedit_server::ClientId;

#[action("Clear locations")]
fn clear_locations(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.locations.clear();
}

#[action("Show locations")]
fn show_locations(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.locations.show = true;
    win.focus_to(Focus::Locations);
}

#[action("Close locations")]
fn close_locations(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.locations.show = false;
    win.focus_to(Focus::Window);
}

#[action("Next location entry")]
fn next_loc_entry(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.locations.select_next();
}

#[action("Previous location entry")]
fn prev_loc_entry(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.locations.select_prev();
}

#[action("Press location entry")]
fn goto_loc_entry(editor: &mut Editor, id: ClientId) {
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

                log::info!("off: {offset}");
                let parent = get!(win.locations.parent_of_selected());
                let path = parent.path().to_path_buf();
                if let Err(e) = editor.open_file(id, &path) {
                    let (win, _buf) = editor.win_buf_mut(id);
                    win.error_msg(&format!("Failed to open file: {e}"));
                    return;
                }

                let (win, buf) = editor.win_buf_mut(id);
                win.goto_offset(offset, buf);
                win.focus_to(Focus::Window);
            }
        }
    }
}

#[action("Goto parent group entry")]
fn select_loc_parent(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.locations.select_parent();
}

#[action("Expand or collapse all location entries")]
fn toggle_all_expand_locs(editor: &mut Editor, id: ClientId) {
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
}

#[action("Keep locations with")]
fn keep_locations(editor: &mut Editor, id: ClientId) {
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
}

#[action("Reject locations with")]
fn reject_locations(editor: &mut Editor, id: ClientId) {
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
}
