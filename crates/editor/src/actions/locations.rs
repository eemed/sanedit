use sanedit_utils::either::Either;

use crate::{
    editor::{
        windows::{Focus, Location},
        Editor,
    },
    server::ClientId,
};

#[action("Show locations")]
fn show(editor: &mut Editor, id: ClientId) {
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
            Location::Group { expanded, .. } => {
                *expanded = !*expanded;
            }
            Location::Item {
                highlights,
                absolute_offset,
                ..
            } => {
                let hl_off = highlights.get(0).map_or(0, |r| r.start);
                let offset = absolute_offset.unwrap_or(0) as usize + hl_off;

                if let Some(parent) = win.locations.parent_of_selected() {
                    if let Either::Left(path) = parent.data().clone() {
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
}
