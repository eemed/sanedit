use std::{cmp::min, sync::Arc};

use crate::editor::{
    filetree::Kind,
    windows::{Focus, Prompt},
    Editor,
};

use sanedit_server::ClientId;

#[action("Show filetree")]
fn show_filetree(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = win_buf!(editor, id);
    if win.ft_view.show {
        win.focus_to(Focus::Filetree);
        return;
    }

    let visible = editor.filetree.iter().count();
    win.ft_view.selection = min(visible - 1, win.ft_view.selection);
    win.ft_view.show = true;
    win.focus_to(Focus::Filetree);
}

#[action("Press filetree entry")]
fn goto_ft_entry(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf(id);
    let path = editor
        .filetree
        .iter()
        .nth(win.ft_view.selection)
        .map(|f| f.path().to_path_buf());

    if let Some(path) = path {
        if let Some(mut node) = editor.filetree.get_mut(&path) {
            match node.kind() {
                Kind::Directory => {
                    if node.is_dir_expanded() {
                        node.collapse();
                    } else {
                        let _ = node.expand();
                    }
                }
                Kind::File => {
                    if let Err(_e) = editor.open_file(id, path) {
                        let (win, _buf) = editor.win_buf_mut(id);
                        win.error_msg("failed to open file {path:?}: {e}");
                    }
                }
            }
        }
    }
}

#[action("Next filetree entry")]
fn next_ft_entry(editor: &mut Editor, id: ClientId) {
    let visible = editor.filetree.iter().count();

    let (win, _buf) = editor.win_buf_mut(id);
    win.ft_view.selection = min(visible - 1, win.ft_view.selection + 1);
}

#[action("Previous filetree entry")]
fn prev_ft_entry(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.ft_view.selection = win.ft_view.selection.saturating_sub(1);
}

#[action("Close filetree")]
fn close_filetree(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.ft_view.show = false;
    win.focus_to(Focus::Window);
}

#[action("Create a new file")]
fn ft_new_file(editor: &mut Editor, id: ClientId) {
    let dir = {
        let (win, _buf) = editor.win_buf(id);
        let Some(entry) = editor.filetree.iter().nth(win.ft_view.selection) else {
            return;
        };

        match entry.kind() {
            Kind::File => {
                let Some(parent) = editor.filetree.parent_of(entry.path()) else {
                    return;
                };
                parent.path().to_path_buf()
            }
            Kind::Directory { .. } => entry.path().to_path_buf(),
        }
    };

    let (win, _buf) = editor.win_buf_mut(id);
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
                win.warn_msg(&format!("Failed to create file {e}"));
                return;
            }

            // Refresh to show new file on tree
            if let Some(mut node) = editor.filetree.get_mut(&dir) {
                let _ = node.refresh();
            }

            // Select the new entry if visible
            if let Some(pos) = editor
                .filetree
                .iter()
                .position(|entry| entry.path() == file)
            {
                let (win, _buf) = editor.win_buf_mut(id);
                win.ft_view.selection = pos;
            }

            // Open the file
            let _ = editor.open_file(id, &file);
        })
        .build();
    win.focus_to(Focus::Prompt);
}

#[action("Go to previous directory")]
fn ft_delete_file(editor: &mut Editor, id: ClientId) {
    let (kind, path) = {
        let (win, _buf) = editor.win_buf(id);
        let Some(entry) = editor.filetree.iter().nth(win.ft_view.selection) else {
            return;
        };
        let path = entry.path().to_path_buf();
        let kind = entry.kind();

        (kind, Arc::new(path))
    };

    let prompt = {
        let kind = match kind {
            Kind::Directory => "directory",
            Kind::File => "file",
        };
        let path = path.strip_prefix(editor.working_dir()).unwrap_or(&path);
        format!("Delete {} {:?}? (y/N)", kind, path)
    };

    let (win, _buf) = editor.win_buf_mut(id);
    win.prompt = Prompt::builder()
        .prompt(&prompt)
        .simple()
        .on_confirm(move |editor, id, input| {
            let yes = input == "y";
            if !yes {
                return;
            }

            let result = match kind {
                Kind::Directory => std::fs::remove_dir_all(path.as_path()),
                Kind::File => std::fs::remove_file(path.as_path()),
            };

            if let Err(e) = result {
                log::error!("Failed to delete file: {e}");
                return;
            }

            if let Some(parent) = path.parent() {
                if let Some(mut node) = editor.filetree.get_mut(parent) {
                    let _ = node.refresh();
                }
            }

            prev_ft_entry.execute(editor, id);

            let (win, _buf) = editor.win_buf_mut(id);
            win.focus_to(Focus::Filetree);
        })
        .build();
    win.focus_to(Focus::Prompt);
}

#[action("Select parent")]
fn select_ft_parent(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf(id);
    let entry = get!(editor.filetree.iter().nth(win.ft_view.selection));
    let parent = get!(editor.filetree.parent_of(entry.path()));
    let pos = get!(editor
        .filetree
        .iter()
        .position(|entry| entry.path() == parent.path()));
    let (win, _buf) = editor.win_buf_mut(id);
    win.ft_view.selection = pos;
}

#[action("Show current file in filetree")]
fn ft_goto_current_file(editor: &mut Editor, id: ClientId) {
    let (win, buf) = win_buf!(editor, id);
    let path = get!(buf.path());
    win.ft_view.selection = get!(editor.filetree.select(path));
}

// #[action("Search for an entry in filetree")]
// fn search_forward(editor: &mut Editor, id: ClientId) {}

// #[action("Search for an entry in filetree backwards")]
// fn search_backwards(editor: &mut Editor, id: ClientId) {}
