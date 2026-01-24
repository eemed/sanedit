use crate::{
    common::is_yes,
    editor::{
        config::Config,
        hooks::Hook,
        ignore::Ignore,
        windows::{Focus, Prompt},
        Editor,
    },
};
use sanedit_server::ClientId;

use super::{
    hooks::run,
    prompt::unsaved_changes,
    window::{focus, mode_normal},
    ActionResult,
};

#[action("Editor: Quit")]
fn quit(editor: &mut Editor, id: ClientId) -> ActionResult {
    // If is the first client
    if !editor.is_last_client() {
        editor.quit_client(id);
        return ActionResult::Ok;
    }

    if editor.buffers.any_unsaved_changes().is_some() {
        unsaved_changes(editor, id, |editor, _id| {
            editor.quit();
            ActionResult::Ok
        })
    } else {
        editor.quit();
    }

    ActionResult::Ok
}

#[action("Editor: Copy to next line end clipboard")]
fn copy_to_eol(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    if win.cursors.has_selections() {
        return ActionResult::Skipped;
    }

    editor.copy_to_eol_to_clipboard(id);
    ActionResult::Ok
}

#[action("Editor: Copy to clipboard")]
fn copy(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    if !win.cursors.has_selections() {
        editor.copy_line_to_clipboard(id);
    } else {
        editor.copy_to_clipboard(id);
    }

    let (win, _buf) = editor.win_buf_mut(id);
    win.stop_selection();
    mode_normal(editor, id);
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

    mode_normal(editor, id);
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
            let input = getf!(out.text());
            let yes = input.is_empty() || is_yes(input);
            if !yes {
                return ActionResult::Failed;
            }

            let path = editor.config_dir.config();
            if let Err(e) = Config::serialize_default_configuration(&path) {
                let (win, _buf) = editor.win_buf_mut(id);
                win.warn_msg("Failed to create default configuration file.");
                log::error!("Failed to create default configuration file to: {path:?} {e}");
                return ActionResult::Failed;
            }

            let _ = editor.open_file(id, &path);
            ActionResult::Ok
        })
        .build();
    focus(editor, id, Focus::Prompt);
}

#[action("Buffer: New scratch buffer")]
fn open_new_scratch_buffer(editor: &mut Editor, id: ClientId) -> ActionResult {
    let bid = editor.buffers_mut().new_scratch();
    editor.open_buffer(id, bid);
    ActionResult::Ok
}

#[action("Editor: Do nothing")]
fn nop(_editor: &mut Editor, _id: ClientId) -> ActionResult {
    ActionResult::Ok
}

#[action("Editor: Toggle ignore")]
fn toggle_ignore(editor: &mut Editor, id: ClientId) -> ActionResult {
    if editor.ignore.is_empty() {
        editor.ignore = Ignore::new(editor.working_dir(), &editor.config, &editor.project_config);
        let (win, _buf) = editor.win_buf_mut(id);
        win.info_msg("File ignoring enabled");
    } else {
        editor.ignore = Ignore::empty();
        let (win, _buf) = editor.win_buf_mut(id);
        win.info_msg("File ignoring disabled");
    }
    ActionResult::Ok
}

#[action("Editor: Load language")]
fn load_language(editor: &mut Editor, id: ClientId) -> ActionResult {
    let bid = editor
        .hooks
        .running_hook()
        .and_then(Hook::buffer_id)
        .unwrap_or_else(|| {
            let (win, _) = editor.win_buf(id);
            win.buffer_id()
        });
    let buf = editor.buffers().get(bid).unwrap();
    let lang = getf!(buf.language.clone());
    editor.load_language(&lang, false);
    ActionResult::Ok
}
