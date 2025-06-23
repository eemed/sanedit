use std::{
    cmp::min,
    path::{Path, PathBuf, MAIN_SEPARATOR},
    sync::Arc,
};

use crate::{
    common::is_yes,
    editor::{
        filetree::{Filetree, Kind},
        windows::{Focus, Prompt},
        Editor,
    },
};

use anyhow::bail;
use sanedit_server::ClientId;

use super::{text::save, window::focus, ActionResult};

#[action("Filetree: Show")]
fn show_filetree(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = win_buf!(editor, id);
    if win.ft_view.show {
        focus(editor, id, Focus::Filetree);
        return ActionResult::Ok;
    }

    let visible = editor.filetree.iter().count();
    win.ft_view.selection = min(visible - 1, win.ft_view.selection);
    win.ft_view.show = true;
    focus(editor, id, Focus::Filetree);

    ft_goto_current_file.execute(editor, id);

    ActionResult::Ok
}

#[action("Filetree: Focus")]
fn focus_filetree(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = win_buf!(editor, id);
    if win.ft_view.show {
        focus(editor, id, Focus::Filetree);
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

                    focus(editor, id, Focus::Window);
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
    focus(editor, id, Focus::Window);

    ActionResult::Ok
}

fn new_file_prefill_path(wd: &Path, selection: usize, filetree: &Filetree) -> Option<String> {
    let dir = {
        let entry = filetree.iter().nth(selection)?;
        match entry.kind() {
            Kind::File => {
                let parent = filetree.parent_of(entry.path())?;
                parent.path().to_path_buf()
            }
            Kind::Directory { .. } => entry.path().to_path_buf(),
        }
    };
    let dir_name = {
        let mut name = dir
            .strip_prefix(wd)
            .unwrap_or(dir.as_path())
            .to_string_lossy()
            .to_string();
        if !name.is_empty() && !name.ends_with(MAIN_SEPARATOR) {
            name.push(MAIN_SEPARATOR);
        }
        name
    };
    Some(dir_name)
}

/// Get a path to a new relative file from userinput.
fn new_file_result_path(wd: &Path, input: &str) -> anyhow::Result<PathBuf> {
    let new = PathBuf::from(input);
    if !new.is_relative() {
        bail!("Path is not relative")
    }

    let absolute = wd.join(new);

    if absolute.exists() {
        bail!("File already exists")
    }

    Ok(absolute)
}

#[action("Filetree: Create new file")]
fn ft_new_file(editor: &mut Editor, id: ClientId) -> ActionResult {
    let root = editor.working_dir().to_path_buf();
    let (win, _buf) = editor.win_buf(id);
    let dir_name = getf!(new_file_prefill_path(
        &root,
        win.ft_view.selection,
        &editor.filetree
    ));

    let (win, _buf) = editor.win_buf_mut(id);
    win.prompt = Prompt::builder()
        .prompt("Filename")
        .input(&dir_name)
        .simple()
        .on_confirm(move |editor, id, out| {
            let input = getf!(out.text());
            let file = match new_file_result_path(&root, input) {
                Ok(path) => path,
                Err(e) => {
                    let (win, _buf) = editor.win_buf_mut(id);
                    win.error_msg(&format!("Invalid path {e}"));
                    return ActionResult::Failed;
                }
            };

            // Create directories leading up to the file
            if let Some(parent) = file.parent() {
                let _ = std::fs::create_dir_all(parent);
            }

            // Create new file
            if let Err(e) = std::fs::File::create_new(&file) {
                let (win, _buf) = editor.win_buf_mut(id);
                win.error_msg(&format!("Failed to create file {e}"));
                return ActionResult::Failed;
            }

            // Refresh to show new file on tree
            let _ = editor.filetree.refresh();

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
            ActionResult::Ok
        })
        .build();
    focus(editor, id, Focus::Prompt);
    ActionResult::Ok
}

#[action("Filetree: Rename file or folder")]
fn ft_rename_file(editor: &mut Editor, id: ClientId) -> ActionResult {
    let old = {
        let (win, _buf) = editor.win_buf(id);
        let entry = getf!(editor.filetree.iter().nth(win.ft_view.selection));
        entry.path().to_path_buf()
    };
    let old_name = old
        .strip_prefix(editor.working_dir())
        .unwrap_or(old.as_path())
        .to_string_lossy();

    let (win, _buf) = editor.win_buf_mut(id);
    win.prompt = Prompt::builder()
        .prompt("Rename")
        .input(&old_name)
        .simple()
        .on_confirm(move |editor, id, out| {
            let input = getf!(out.text());
            let new = match new_file_result_path(editor.working_dir(), input) {
                Ok(path) => path,
                Err(e) => {
                    let (win, _buf) = editor.win_buf_mut(id);
                    win.error_msg(&format!("Rename error: {e}"));
                    return ActionResult::Failed;
                }
            };

            // Create directories leading up to the renamed thing
            if let Some(parent) = new.parent() {
                let _ = std::fs::create_dir_all(parent);
            }

            // Rename
            match editor.buffers_mut().find(&old) {
                Some(bid) => {
                    let buf = editor.buffers_mut().get_mut(bid).unwrap();
                    buf.set_path(&new);
                    save.execute(editor, id);

                    if old.is_file() {
                        let _ = std::fs::remove_file(&old);
                    } else {
                        let _ = std::fs::remove_dir_all(&old);
                    }
                }
                None => {
                    if let Err(e) = std::fs::rename(&old, &new) {
                        let (win, _buf) = editor.win_buf_mut(id);
                        win.warn_msg(&format!("Failed to rename file/dir {e}"));
                        return ActionResult::Failed;
                    }
                }
            }

            // Refresh tree to show moved stuff
            let _ = editor.filetree.refresh();

            // Select the new entry if visible
            if let Some(pos) = editor
                .filetree
                .iter()
                .position(|entry| entry.path() == new.as_path())
            {
                let (win, _buf) = editor.win_buf_mut(id);
                win.ft_view.selection = pos;
            }

            focus(editor, id, Focus::Filetree);
            ActionResult::Ok
        })
        .build();
    focus(editor, id, Focus::Prompt);
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
            let ans = getf!(out.text());
            if !is_yes(ans) {
                return ActionResult::Failed;
            }

            let result = match kind {
                Kind::Directory => std::fs::remove_dir_all(path.as_path()),
                Kind::File => std::fs::remove_file(path.as_path()),
            };

            if let Err(e) = result {
                log::error!("Failed to delete file: {e}");
                return ActionResult::Failed;
            }

            // Refresh tree
            if let Some(parent) = path.parent() {
                if let Some(mut node) = editor.filetree.get_mut(parent) {
                    let _ = node.refresh();
                }
            }

            prev_ft_entry.execute(editor, id);
            focus(editor, id, Focus::Filetree);

            // Delete buffer if exists
            if let Some(bid) = editor.buffers.find(path.as_path()) {
                let _ = editor.remove_buffer(id, bid);
            }
            ActionResult::Ok
        })
        .build();

    focus(editor, id, Focus::Prompt);
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
