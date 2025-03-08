use std::process::Command;

use crate::editor::{
    windows::{PromptOutput, ShellKind},
    Editor,
};

use sanedit_server::ClientId;

use super::{jobs::TmuxShellCommand, ActionResult};

pub(crate) fn execute_prompt(editor: &mut Editor, id: ClientId, out: PromptOutput) {
    let cmd = get!(out.text());
    execute(editor, id, true, cmd);
}

pub(crate) fn execute(editor: &mut Editor, id: ClientId, interactive: bool, cmd: &str) -> ActionResult {
    let shell = editor.config.editor.shell.clone();
    let (win, _buf) = editor.win_buf(id);

    if !interactive {
        run_non_interactive(&shell, &cmd);
        return ActionResult::Ok;
    }

    match &win.shell_kind {
        ShellKind::Tmux { pane } => {
            let mut job = TmuxShellCommand::new(id, &shell, cmd);
            if let Some(pane) = pane {
                job = job.pane(pane.clone());
            }

            editor.job_broker.request(job);
        }
        ShellKind::NonInteractive => run_non_interactive(&shell, cmd),
    }

    ActionResult::Ok
}

fn run_non_interactive(shell: &str, cmd: &str) {
    let _ = Command::new(shell).args(["-c", cmd]).spawn();
}
