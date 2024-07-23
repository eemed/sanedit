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
    win.focus = Focus::Locations;
}

#[action("Close locations")]
fn close(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
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
        // TODO vec<u8>
        match sel {
            Location::Group {
                name,
                expanded,
                locations,
            } => {
                *expanded = !*expanded;
            }
            Location::Item {
                name,
                line,
                column,
                highlights,
            } => log::info!("Open: {name}"),
        }
    }
}
