use crate::editor::{
    windows::{PromptOutput, ShellKind},
    Editor,
};

use sanedit_server::ClientId;

use super::jobs::TmuxShellCommand;

pub(crate) fn execute_prompt(editor: &mut Editor, id: ClientId, out: PromptOutput) {
    let cmd = get!(out.text());
    execute(editor, id, cmd);
}

pub(crate) fn execute(editor: &mut Editor, id: ClientId, cmd: &str) {
    let shell = editor.config.editor.shell.clone();
    let (win, _buf) = editor.win_buf(id);

    match &win.shell_kind {
        ShellKind::Tmux { pane } => {
            let mut job = TmuxShellCommand::new(id, &shell, cmd);
            if let Some(pane) = pane {
                job = job.pane(pane.clone());
            }

            editor.job_broker.request(job);
        }
    }
}
