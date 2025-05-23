use std::process::Command;

use crate::editor::{
    windows::{PromptOutput, WindowManager},
    Editor,
};

use sanedit_server::ClientId;

use super::{jobs::TmuxShellCommand, ActionResult};

pub(crate) fn execute_prompt(editor: &mut Editor, id: ClientId, out: PromptOutput) -> ActionResult {
    let cmd = getf!(out.text());
    execute(editor, id, true, cmd)
}

pub(crate) fn execute(
    editor: &mut Editor,
    id: ClientId,
    interactive: bool,
    cmd: &str,
) -> ActionResult {
    let shell = editor.config.editor.shell.clone();
    let (win, _buf) = editor.win_buf(id);

    if !interactive {
        run_non_interactive(&shell, &cmd);
        return ActionResult::Ok;
    }

    match &win.window_manager {
        WindowManager::Tmux { shell_pane } => {
            let mut job = TmuxShellCommand::new(id, &shell, cmd);
            if let Some(pane) = shell_pane {
                job = job.pane(pane.clone());
            }

            editor.job_broker.request(job);
        }
        WindowManager::Wezterm => todo!(),
    }

    ActionResult::Ok
}

fn run_non_interactive(shell: &str, cmd: &str) {
    let _ = Command::new(shell).args(["-c", cmd]).spawn();
}
