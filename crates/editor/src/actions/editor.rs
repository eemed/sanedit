use crate::{
    common::is_yes,
    editor::{
        config::Config,
        hooks::Hook,
        windows::{Focus, Prompt},
        Editor,
    },
};
use sanedit_server::ClientId;

use super::{hooks::run, prompt::unsaved_changes, window::focus, ActionResult};

#[action("Editor: Quit")]
fn quit(editor: &mut Editor, id: ClientId) -> ActionResult {
    // If is the first client
    if id.0 == 0 {
        if editor.buffers.any_unsaved_changes().is_some() {
            unsaved_changes(editor, id, |editor, id| {
                editor.quit_client(id);
            })
        } else {
            editor.quit();
        }
    } else {
        editor.quit_client(id);
    }

    ActionResult::Ok
}

#[action("Editor: Copy to clipboard")]
fn copy(editor: &mut Editor, id: ClientId) -> ActionResult {
    editor.copy_to_clipboard(id);
    ActionResult::Ok
}

#[action("Editor: Paste from clipboard")]
fn paste(editor: &mut Editor, id: ClientId) -> ActionResult {
    editor.paste_from_clipboard(id);
    ActionResult::Ok
}

#[action("Editor: Cut to clipboard")]
fn cut(editor: &mut Editor, id: ClientId) -> ActionResult {
    editor.copy_to_clipboard(id);

    run(editor, id, Hook::RemovePre);
    let (win, buf) = editor.win_buf_mut(id);
    let bid = buf.id;
    if win.remove_cursor_selections(buf).unwrap_or(false) {
        run(editor, id, Hook::BufChanged(bid));
    }

    ActionResult::Ok
}

#[action("Editor: Open configuration file")]
fn open_config(editor: &mut Editor, id: ClientId) -> ActionResult {
    let config = editor.config_dir.config();
    if !config.exists() {
        prompt_create_and_open_config(editor, id);
        ActionResult::Ok
    } else {
        editor.open_file(id, &config).into()
    }
}

fn prompt_create_and_open_config(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);

    win.prompt = Prompt::builder()
        .prompt("Configuration file is missing. Create default configuration? (Y/n)")
        .simple()
        .on_confirm(|editor, id, out| {
            let input = get!(out.text());
            let yes = input.is_empty() || is_yes(input);
            if !yes {
                return;
            }

            let path = editor.config_dir.config();
            if let Err(e) = Config::serialize_default_configuration(&path) {
                let (win, _buf) = editor.win_buf_mut(id);
                win.warn_msg("Failed to create default configuration file.");
                log::error!("Failed to create default configuration file to: {path:?} {e}");
                return;
            }

            let _ = editor.open_file(id, &path);
        })
        .build();
    focus(editor, id, Focus::Prompt);
}

#[action("Buffer: New scratch buffer")]
fn open_new_scratch_buffer(editor: &mut Editor, id: ClientId) -> ActionResult {
    let bid = editor.buffers_mut().new_scratch();
    let (win, _buf) = editor.win_buf_mut(id);
    win.open_buffer(bid);
    run(editor, id, Hook::BufEnter(bid));
    ActionResult::Ok
}

#[action("Editor: Do nothing")]
fn nop(_editor: &mut Editor, _id: ClientId) -> ActionResult {
    ActionResult::Ok
}

#[action("Editor: Load filetype")]
fn load_filetype(editor: &mut Editor, id: ClientId) -> ActionResult {
    let bid = editor
        .hooks
        .running_hook()
        .and_then(Hook::buffer_id)
        .unwrap_or_else(|| {
            let (win, _) = editor.win_buf(id);
            win.buffer_id()
        });
    let buf = editor.buffers().get(bid).unwrap();
    let ft = getf!(buf.filetype.clone());
    editor.load_filetype(&ft, false);
    ActionResult::Ok
}
