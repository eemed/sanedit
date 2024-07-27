use std::{cmp::min, sync::Arc};

use crate::{
    editor::{
        filetree::{Kind, Node},
        windows::{Focus, Prompt},
        Editor,
    },
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

#[action("Press filetree entry")]
fn confirm(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf(id);
    let path = editor
        .filetree
        .iter()
        .nth(win.ft_view.selection)
        .map(|f| f.path().to_path_buf());

    if let Some(path) = path {
        if let Some(node) = editor.filetree.get_mut(&path) {
            match node.kind() {
                Kind::Directory => {
                    if node.is_dir_expanded() {
                        node.collapse();
                    } else {
                        node.expand();
                    }
                }
                Kind::File => {
                    if let Err(e) = editor.open_file(id, path) {
                        let (win, buf) = editor.win_buf_mut(id);
                        win.error_msg("failed to open file {path:?}: {e}");
                    }
                }
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
    let (win, buf) = editor.win_buf_mut(id);
    win.ft_view.show = false;
    win.focus = Focus::Window;
}

#[action("Create a new file")]
fn create_new_file(editor: &mut Editor, id: ClientId) {
    let dir = {
        let (win, _buf) = editor.win_buf(id);
        let Some(entry) = editor.filetree.iter().nth(win.ft_view.selection) else {
            return;
        };

        match entry.node().kind() {
            Kind::File => {
                let Some(parent) = editor.filetree.parent_of(entry.path()) else {
                    return;
                };
                parent.path().to_path_buf()
            }
            Kind::Directory { .. } => entry.path().to_path_buf(),
        }
    };

    let (win, buf) = editor.win_buf_mut(id);
    win.prompt = Prompt::builder()
        .prompt("Filename")
        .simple()
        .on_confirm(move |editor, id, input| {
            let file = dir.join(input);

            // Create directories leading up to the file
            if let Some(parent) = file.parent() {
                let _ = std::fs::create_dir_all(parent);
            }

            // Create new file
            if let Err(e) = std::fs::File::create_new(&file) {
                let (win, _buf) = editor.win_buf_mut(id);
                win.warn_msg("Failed to create file {e}");
                return;
            }

            // Refresh to show new file on tree
            if let Some(node) = editor.filetree.get_mut(&dir) {
                node.refresh();
            }

            // Select the new entry
            let pos = editor
                .filetree
                .iter()
                .position(|entry| entry.node().path() == file)
                .unwrap();
            let (win, _buf) = editor.win_buf_mut(id);
            win.ft_view.selection = pos;

            // Open the file
            editor.open_file(id, &file);
        })
        .build();
    win.focus = Focus::Prompt;
}

#[action("Go to previous directory")]
fn delete_file(editor: &mut Editor, id: ClientId) {}

// #[action("Search for an entry in filetree")]
// fn search_forward(editor: &mut Editor, id: ClientId) {}

// #[action("Search for an entry in filetree backwards")]
// fn search_backwards(editor: &mut Editor, id: ClientId) {}
