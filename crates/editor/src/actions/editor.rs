use crate::{editor::Editor, server::ClientId};

use super::shell;

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
