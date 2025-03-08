use std::{cmp::min, sync::Arc};

use crate::{
    common::is_yes,
    editor::{
        filetree::Kind,
        windows::{Focus, Prompt},
        Editor,
    },
};

use sanedit_server::ClientId;

use super::ActionResult;

#[action("Filetree: Show")]
fn show_filetree(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = win_buf!(editor, id);
    if win.ft_view.show {
        win.focus_to(Focus::Filetree);
        return ActionResult::Ok;
    }

    let visible = editor.filetree.iter().count();
    win.ft_view.selection = min(visible - 1, win.ft_view.selection);
    win.ft_view.show = true;
    win.focus_to(Focus::Filetree);
    ActionResult::Ok
}

#[action("Filetree: Focus")]
fn focus_filetree(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = win_buf!(editor, id);
    if win.ft_view.show {
        win.focus_to(Focus::Filetree);
    }

    ActionResult::Ok
}

#[action("Filetree: Confirm entry")]
fn goto_ft_entry(editor: &mut Editor, id: ClientId) -> ActionResult {
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
                        return ActionResult::Failed;
                    }

                    let (win, _buf) = editor.win_buf_mut(id);
                    win.focus_to(Focus::Window);
                }
            }
        }
    }

    ActionResult::Ok
}

#[action("Filetree: Next entry")]
fn next_ft_entry(editor: &mut Editor, id: ClientId) -> ActionResult {
    let visible = editor.filetree.iter().count();
    let (win, _buf) = editor.win_buf_mut(id);
    win.ft_view.selection = min(visible - 1, win.ft_view.selection + 1);

    ActionResult::Ok
}

#[action("Filetree: Previous entry")]
fn prev_ft_entry(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    win.ft_view.selection = win.ft_view.selection.saturating_sub(1);
    ActionResult::Ok
}

#[action("Filetree: Close")]
fn close_filetree(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    win.ft_view.show = false;
    win.focus_to(Focus::Window);

    ActionResult::Ok
}

#[action("Filetree: Create new file")]
fn ft_new_file(editor: &mut Editor, id: ClientId) -> ActionResult {
    let dir = {
        let (win, _buf) = editor.win_buf(id);
        let entry = getf!(editor.filetree.iter().nth(win.ft_view.selection));

        match entry.kind() {
            Kind::File => {
                let parent = getf!(editor.filetree.parent_of(entry.path()));
                parent.path().to_path_buf()
            }
            Kind::Directory { .. } => entry.path().to_path_buf(),
        }
    };

    let (win, _buf) = editor.win_buf_mut(id);
    win.prompt = Prompt::builder()
        .prompt("Filename")
        .simple()
        .on_confirm(move |editor, id, out| {
            let path = get!(out.text());
            let file = dir.join(path);

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
    ActionResult::Ok
}

#[action("Filetree: Delete file")]
fn ft_delete_file(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (kind, path) = {
        let (win, _buf) = editor.win_buf(id);
        let entry = getf!(editor.filetree.iter().nth(win.ft_view.selection));
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
        .on_confirm(move |editor, id, out| {
            let ans = get!(out.text());
            if !is_yes(ans) {
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

    ActionResult::Ok
}

#[action("Filetree: Goto parent")]
fn select_ft_parent(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf(id);
    let entry = getf!(editor.filetree.iter().nth(win.ft_view.selection));
    let parent = getf!(editor.filetree.parent_of(entry.path()));
    let pos = getf!(editor
        .filetree
        .iter()
        .position(|entry| entry.path() == parent.path()));
    let (win, _buf) = editor.win_buf_mut(id);
    win.ft_view.selection = pos;

    ActionResult::Ok
}

#[action("Filetree: Show current file")]
fn ft_goto_current_file(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, buf) = win_buf!(editor, id);
    let path = getf!(buf.path());
    win.ft_view.selection = getf!(editor.filetree.select(path));
    ActionResult::Ok
}

// #[action("Search for an entry in filetree")]
// fn search_forward(editor: &mut Editor, id: ClientId) {}

// #[action("Search for an entry in filetree backwards")]
// fn search_backwards(editor: &mut Editor, id: ClientId) {}
