use crate::{
    editor::{hooks::Hook, Editor},
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
    let cmd = editor.options.project.build_command.clone();
    shell::execute(editor, id, &cmd);
}

#[action("Run project")]
fn run_project(editor: &mut Editor, id: ClientId) {
    let cmd = editor.options.project.run_command.clone();
    shell::execute(editor, id, &cmd);
}

#[action("Copy selection to clipboard")]
fn copy(editor: &mut Editor, id: ClientId) {
    editor.copy_to_clipboard(id);
}

#[action("Paste from clipboard")]
fn paste(editor: &mut Editor, id: ClientId) {
    editor.paste_from_clipboard(id);
}

#[action("Copy selection to clipboard and remove it")]
fn cut(editor: &mut Editor, id: ClientId) {
    editor.copy_to_clipboard(id);

    run(editor, id, Hook::RemovePre);
    let (win, buf) = editor.win_buf_mut(id);
    win.remove_cursor_selections(buf);
    run(editor, id, Hook::BufChanged);
}
