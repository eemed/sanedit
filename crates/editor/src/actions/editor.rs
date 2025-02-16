use crate::{
    common::is_yes,
    editor::{
        buffers::BufferId,
        config::Config,
        hooks::Hook,
        windows::{Focus, Prompt},
        Editor,
    },
};
use sanedit_server::ClientId;

use super::{hooks::run, prompt::unsaved_changes, shell, text::save};

#[action("Quit Sanedit")]
fn quit(editor: &mut Editor, id: ClientId) {
    // If is the first client
    if id.0 == 0 {
        if editor.buffers.any_unsaved_changes().is_some() {
            unsaved_changes(editor, id, |editor, id| {
                let unsaved: Vec<BufferId> = editor
                    .buffers
                    .iter()
                    .filter(|(_, buf)| buf.is_modified())
                    .map(|(bid, _)| bid)
                    .collect();
                for bid in unsaved {
                    let (win, _buf) = win_buf!(editor, id);
                    win.open_buffer(bid);
                    save.execute(editor, id)
                }
                editor.quit_client(id);
            })
        } else {
            editor.quit();
        }
    } else {
        let (_, buf) = editor.win_buf(id);
        if buf.is_modified() {
            unsaved_changes(editor, id, move |editor, id| {
                let (win, buf) = editor.win_buf_mut(id);
                win.open_buffer(buf.id);
                save.execute(editor, id);

                editor.quit_client(id);
            })
        } else {
            editor.quit_client(id);
        }
    }
}

#[action("Build project")]
fn build_project(editor: &mut Editor, id: ClientId) {
    let cmd = editor.config.editor.build_command.clone();
    shell::execute(editor, id, &cmd);
}

#[action("Run project")]
fn run_project(editor: &mut Editor, id: ClientId) {
    let cmd = editor.config.editor.run_command.clone();
    shell::execute(editor, id, &cmd);
}

#[action("Copy selection to clipboard")]
fn copy(editor: &mut Editor, id: ClientId) {
    editor.copy_to_clipboard(id);
}

#[action("Paste from clipboard below the current line")]
fn paste_below(editor: &mut Editor, id: ClientId) {
    editor.paste_from_clipboard_below(id);
}

#[action("Paste from clipboard")]
fn paste(editor: &mut Editor, id: ClientId) {
    editor.paste_from_clipboard(id);
    // let (_, buf) = editor.win_buf_mut(id);
    // let bid = buf.id;
    // run(editor, id, Hook::BufChanged(bid));
    // if win.remove_cursor_selections(buf).unwrap_or(false) {
    // }
}

#[action("Cut selection to clipboard")]
fn cut(editor: &mut Editor, id: ClientId) {
    editor.copy_to_clipboard(id);

    run(editor, id, Hook::RemovePre);
    let (win, buf) = editor.win_buf_mut(id);
    let bid = buf.id;
    if win.remove_cursor_selections(buf).unwrap_or(false) {
        run(editor, id, Hook::BufChanged(bid));
    }
}

#[action("Open SanEdit configuration file")]
fn open_config(editor: &mut Editor, id: ClientId) {
    let config = editor.config_dir.config();
    if !config.exists() {
        prompt_create_and_open_config(editor, id);
    } else {
        let _ = editor.open_file(id, &config);
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
    win.focus_to(Focus::Prompt);
}

#[action("Open a new scratch buffer")]
fn open_new_scratch_buffer(editor: &mut Editor, id: ClientId) {
    let bid = editor.buffers_mut().new_scratch();
    let (win, _buf) = editor.win_buf_mut(id);
    win.open_buffer(bid);
}

#[action("Do nothing")]
fn nop(_editor: &mut Editor, _id: ClientId) {}

#[action("Load filetype")]
fn load_filetype(editor: &mut Editor, id: ClientId) {
    let bid = editor
        .hooks
        .running_hook()
        .and_then(Hook::buffer_id)
        .unwrap_or_else(|| {
            let (win, _) = editor.win_buf(id);
            win.buffer_id()
        });
    let buf = editor.buffers().get(bid).unwrap();
    let Some(ft) = buf.filetype.clone() else {
        return;
    };
    editor.load_filetype(&ft);
}
