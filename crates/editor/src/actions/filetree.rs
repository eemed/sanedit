use std::{
    cmp::min,
    fs, io,
    path::{Path, PathBuf, MAIN_SEPARATOR},
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

use super::{window::focus, ActionResult};

#[action("Filetree: Select first entry")]
fn ft_select_first(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = win_buf!(editor, id);
    win.ft_view.selection = 0;
    ActionResult::Ok
}

#[action("Filetree: Select last entry")]
fn ft_select_last(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = win_buf!(editor, id);
    let max = editor.filetree.iter().count() - 1;
    win.ft_view.selection = max;
    ActionResult::Ok
}

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

#[action("Filetree: Set root")]
fn set_root(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf(id);
    let path = getf!(editor
        .filetree
        .iter()
        .nth(win.ft_view.selection)
        .map(|f| f.path().to_path_buf()));

    let node = getf!(editor.filetree.get_mut(&path));
    if matches!(node.kind(), Kind::Directory) {
        let _ = editor.change_working_dir(&path);
        let (win, _buf) = editor.win_buf_mut(id);
        win.ft_view.selection = 0;
    }
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
    let path = getf!(editor
        .filetree
        .iter()
        .nth(win.ft_view.selection)
        .map(|f| f.path().to_path_buf()));

    let mut node = getf!(editor.filetree.get_mut(&path));
    match node.kind() {
        Kind::Directory => {
            if node.is_dir_expanded() {
                node.collapse();
            } else {
                let _ = node.expand();
                let _ = node.refresh();
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
    let dir_name = prefill_dir(wd, &dir);
    Some(dir_name)
}

fn prefill_dir(root: &Path, path: &Path) -> String {
    let mut name = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string();
    if !name.is_empty() && !name.ends_with(MAIN_SEPARATOR) {
        name.push(MAIN_SEPARATOR);
    }
    name
}

/// Get a path to a new relative file from userinput.
pub fn new_file_result_path(wd: &Path, input: &str) -> anyhow::Result<PathBuf> {
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

pub fn create_new_file(editor: &mut Editor, id: ClientId, name: String) -> ActionResult {
    let root = editor.working_dir().to_path_buf();
    let (win, _buf) = editor.win_buf_mut(id);
    win.prompt = Prompt::builder()
        .prompt("Filename")
        .input(&name)
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

pub fn rename_file(editor: &mut Editor, id: ClientId, old: PathBuf) -> ActionResult {
    let old_name = old
        .strip_prefix(editor.working_dir())
        .unwrap_or(&old)
        .to_string_lossy();
    let (win, _buf) = editor.win_buf_mut(id);
    win.prompt = Prompt::builder()
        .prompt("Rename")
        .input(&old_name)
        .simple()
        .on_confirm(move |editor, id, out| {
            let input = getf!(out.text());
            let root = editor.working_dir();
            let new = match new_file_result_path(root, input) {
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

            let mut bids = vec![];
            for (bid, buf) in editor.buffers.iter() {
                if let Some(bpath) = buf.path() {
                    if bpath == old {
                        bids.push((bid, new.clone()));
                        continue;
                    }

                    match bpath.strip_prefix(&old) {
                        Ok(suffix) => {
                            let new_location = new.join(suffix);
                            bids.push((bid, new_location));
                        }
                        Err(_) => {}
                    }
                }
            }

            for (bid, path) in bids {
                if let Some(buf) = editor.buffers.get_mut(bid) {
                    if let Err(e) = buf.rename(&path) {
                        log::error!("Failed to rename buffer: {} to {path:?}: {e}", buf.name());
                    }
                }
            }

            if let Err(e) = rename(&old, &new) {
                let (win, _buf) = editor.win_buf_mut(id);
                win.warn_msg(&format!("Failed to rename file/dir {e}"));
                return ActionResult::Failed;
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

fn rename(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    if src.as_ref().is_dir() {
        fs::create_dir_all(&dst)?;
        for entry in fs::read_dir(src.as_ref())? {
            let entry = entry?;
            let dst = dst.as_ref().join(entry.file_name());
            rename(entry.path(), dst)?;
        }
        fs::remove_dir(src.as_ref())?;
    } else if !dst.as_ref().exists() {
        if let Err(_) = std::fs::rename(src.as_ref(), dst.as_ref()) {
            fs::copy(src.as_ref(), dst.as_ref())?;
            fs::remove_file(src.as_ref())?;
        }
    } else {
        fs::remove_file(src.as_ref())?;
    }

    Ok(())
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

    create_new_file(editor, id, dir_name)
}

#[action("Filetree: Rename file or folder")]
fn ft_rename_file(editor: &mut Editor, id: ClientId) -> ActionResult {
    let old = {
        let (win, _buf) = editor.win_buf(id);
        let entry = getf!(editor.filetree.iter().nth(win.ft_view.selection));
        entry.path().to_path_buf()
    };
    rename_file(editor, id, old)
}

#[action("Buffer: Rename file")]
fn buffer_rename_file(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (_win, buf) = editor.win_buf_mut(id);
    let initial_path = buf.path().map(PathBuf::from).unwrap_or(PathBuf::new());
    rename_file(editor, id, initial_path)
}

#[action("Buffer: Remove file")]
fn buffer_remove_file(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (_win, buf) = editor.win_buf_mut(id);
    if let Some(path) = buf.path().map(PathBuf::from) {
        return delete_file(editor, id, path);
    }
    ActionResult::Skipped
}

#[action("Buffer: Create file")]
fn buffer_create_file(editor: &mut Editor, id: ClientId) -> ActionResult {
    let root = editor.working_dir();
    let (_win, buf) = editor.win_buf(id);
    let prefill = buf
        .path()
        .map(Path::parent)
        .flatten()
        .map(|parent| prefill_dir(root, parent))
        .unwrap_or(String::new());
    create_new_file(editor, id, prefill)
}

#[action("Filetree: Delete file")]
fn ft_delete_file(editor: &mut Editor, id: ClientId) -> ActionResult {
    let path = {
        let (win, _buf) = editor.win_buf(id);
        let entry = getf!(editor.filetree.iter().nth(win.ft_view.selection));
        let path = entry.path().to_path_buf();
        path
    };
    delete_file(editor, id, path)
}

fn delete_file(editor: &mut Editor, id: ClientId, path: PathBuf) -> ActionResult {
    let prompt = {
        let kind = if path.is_dir() { "directory" } else { "file" };
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

            // Delete buffers
            let mut bids = vec![];
            for (bid, buf) in editor.buffers.iter() {
                if let Some(bpath) = buf.path() {
                    if bpath.strip_prefix(path.as_path()).is_ok() {
                        bids.push(bid);
                    }
                }
            }

            for bid in bids {
                let _ = editor.remove_buffer(id, bid);
            }

            let result = if path.is_dir() {
                std::fs::remove_dir_all(path.as_path())
            } else {
                std::fs::remove_file(path.as_path())
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
