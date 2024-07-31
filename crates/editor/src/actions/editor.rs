use crate::{
    editor::{
        config::{serialize_default_configuration, Config},
        hooks::Hook,
        windows::{Focus, Prompt},
        Editor,
    },
    server::ClientId,
};

use super::{hooks::run, shell};

#[action("Quit Sanedit")]
fn quit(editor: &mut Editor, id: ClientId) {
    // If is the first client
    if id.0 == 0 {
        editor.quit();
    } else {
        editor.quit_client(id);
    }
}

#[action("Build project")]
fn build_project(editor: &mut Editor, id: ClientId) {
    let cmd = editor.options.build_command.clone();
    shell::execute(editor, id, &cmd);
}

#[action("Run project")]
fn run_project(editor: &mut Editor, id: ClientId) {
    let cmd = editor.options.run_command.clone();
    shell::execute(editor, id, &cmd);
}

#[action("Copy selection to clipboard")]
fn copy(editor: &mut Editor, id: ClientId) {
    editor.copy_to_clipboard(id);
    let (win, buf) = editor.win_buf_mut(id);
    for cursor in win.cursors.cursors_mut() {
        cursor.unanchor();
    }
}

#[action("Paste from clipboard")]
fn paste(editor: &mut Editor, id: ClientId) {
    editor.paste_from_clipboard(id);
    let (win, buf) = editor.win_buf_mut(id);
    win.remove_cursor_selections(buf);
}

#[action("Copy selection to clipboard and remove it")]
fn cut(editor: &mut Editor, id: ClientId) {
    editor.copy_to_clipboard(id);

    run(editor, id, Hook::RemovePre);
    let (win, buf) = editor.win_buf_mut(id);
    win.remove_cursor_selections(buf);
    run(editor, id, Hook::BufChanged);
}

#[action("Open configuration file")]
fn open_config(editor: &mut Editor, id: ClientId) {
    let config = editor.config_dir.config();
    if !config.exists() {
        prompt_create_and_open_config(editor, id);
    } else {
        editor.open_file(id, &config);
    }
}

fn prompt_create_and_open_config(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);

    win.prompt = Prompt::builder()
        .prompt("Configuration file is missing. Create default configuration? (Y/n)")
        .simple()
        .on_confirm(|editor, id, input| {
            let yes = input.is_empty() || input.eq_ignore_ascii_case("y");
            if !yes {
                return;
            }

            let path = editor.config_dir.config();
            if let Err(e) = serialize_default_configuration(&path) {
                let (win, buf) = editor.win_buf_mut(id);
                win.warn_msg("Failed to create default configuration file.");
                return;
            }

            editor.open_file(id, &path);
        })
        .build();
    win.focus = Focus::Prompt;
}
