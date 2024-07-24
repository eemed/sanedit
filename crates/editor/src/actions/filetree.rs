use std::cmp::min;

use crate::{
    editor::{filetree::PressResult, windows::Focus, Editor},
    server::ClientId,
};

#[action("Show filetree")]
fn show(editor: &mut Editor, id: ClientId) {
    let visible = editor.filetree.iter().count();
    let (win, buf) = editor.win_buf_mut(id);

    win.ft_view.selection = min(visible - 1, win.ft_view.selection);
    win.ft_view.show = true;
    win.focus = Focus::Filetree;
}

#[action("Next filetree entry")]
fn confirm(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf(id);
    let path = editor
        .filetree
        .iter()
        .nth(win.ft_view.selection)
        .map(|f| f.path);

    if let Some(path) = path {
        if let PressResult::IsFile = editor.filetree.on_press(&path) {
            if let Err(e) = editor.open_file(id, path) {
                let (win, buf) = editor.win_buf_mut(id);
                win.error_msg("failed to open file {path:?}: {e}");
            }
        }
    }
}

#[action("Next filetree entry")]
fn next_entry(editor: &mut Editor, id: ClientId) {
    let visible = editor.filetree.iter().count();

    let (win, buf) = editor.win_buf_mut(id);
    win.ft_view.selection = min(visible - 1, win.ft_view.selection + 1);
}

#[action("Previous filetree entry")]
fn prev_entry(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    win.ft_view.selection = win.ft_view.selection.saturating_sub(1);
}

#[action("Close filetree")]
fn close(editor: &mut Editor, id: ClientId) {
    log::info!("Close ft");
    let (win, buf) = editor.win_buf_mut(id);
    win.ft_view.show = false;
    win.focus = Focus::Window;
}

#[action("Search for an entry in filetree")]
fn search_forward(editor: &mut Editor, id: ClientId) {}

#[action("Search for an entry in filetree backwards")]
fn search_backwards(editor: &mut Editor, id: ClientId) {}
