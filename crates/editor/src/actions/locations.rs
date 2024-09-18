use sanedit_utils::either::Either;

use crate::editor::{windows::Focus, Editor};

use sanedit_server::ClientId;

#[action("Clear locations")]
fn clear(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    win.locations.clear();
}

#[action("Show locations")]
fn show_locations(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    win.locations.show = true;
    win.focus = Focus::Locations;
}

#[action("Close locations")]
fn close(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    win.locations.show = false;
    win.focus = Focus::Window;
}

#[action("Next location entry")]
fn next_entry(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    win.locations.select_next();
}

#[action("Previous location entry")]
fn prev_entry(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    win.locations.select_prev();
}

#[action("Press location entry")]
fn confirm(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);

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
                let hl_off = item.highlights().get(0).map_or(0, |r| r.start);
                let offset = item.absolute_offset().unwrap_or(0) + hl_off as u64;

                if let Some(parent) = win.locations.parent_of_selected() {
                    let path = parent.path().to_path_buf();
                    if let Err(e) = editor.open_file(id, &path) {
                        let (win, _buf) = editor.win_buf_mut(id);
                        win.error_msg(&format!("Failed to open file: {e}"));
                        return;
                    }

                    let (win, buf) = editor.win_buf_mut(id);
                    win.goto_offset(offset, buf);
                }
            }
        }
    }
}

#[action("Goto parent group entry")]
fn select_parent(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    win.locations.select_parent();
}

#[action("Expand or collapse all location entries")]
fn toggle_expand_all(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
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
