use std::cmp::min;

use crate::{
    editor::{filetree::PressResult, windows::Focus, Editor},
    server::ClientId,
};

#[action("Show filetree")]
fn show_filetree(editor: &mut Editor, id: ClientId) {
    let visible = editor.filetree.iter().count();
    let (win, buf) = editor.win_buf_mut(id);

    win.ft_view.selection = min(visible - 1, win.ft_view.selection);
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
                win.error_msg("Failed to open file {path:?}: {e}");
            }
        }
    }
}

#[action("Next filetree entry")]
fn next_entry(editor: &mut Editor, id: ClientId) {
    let visible = editor.filetree.iter().count();

    let (win, buf) = editor.win_buf_mut(id);
    win.ft_view.selection = min(visible - 1, win.ft_view.selection + 1);

    // TODO ensure visible
}

#[action("Previous filetree entry")]
fn prev_entry(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    win.ft_view.selection = win.ft_view.selection.saturating_sub(1);

    // TODO ensure visible
}

#[action("Close filetree")]
fn close_filetree(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    win.focus = Focus::Window;
}

#[action("Scroll up filetree")]
fn scroll_up(editor: &mut Editor, id: ClientId) {}

#[action("Scroll up filetree")]
fn scroll_down(editor: &mut Editor, id: ClientId) {}
