use std::process::Command;

use crate::editor::{
    job_broker::KeepInTouch,
    windows::{PromptOutput, WindowManager},
    Editor,
};

use sanedit_server::{ClientId, Job, JobContext, JobResult};

use super::ActionResult;

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
    let (win, _buf) = editor.win_buf_mut(id);

    if !interactive {
        run_non_interactive(&shell, &cmd);
        return ActionResult::Ok;
    }

    let job = ShellCommand {
        client_id: id,
        command: cmd.into(),
        shell,
        win_manager: win.window_manager.clone(),
    };
    editor.job_broker.request(job);

    ActionResult::Ok
}

fn run_non_interactive(shell: &str, cmd: &str) {
    let _ = Command::new(shell).args(["-c", cmd]).spawn();
}

#[derive(Clone, Debug)]
pub(crate) struct ShellCommand {
    client_id: ClientId,
    command: String,
    shell: String,
    win_manager: WindowManager,
}

fn escape_cmd(command: &str) -> String {
    let mut result = String::new();
    for ch in command.chars() {
        match ch {
            '\'' => result.push_str("'\\''"),
            c => result.push(c),
        }
    }
    result
}

impl Job for ShellCommand {
    fn run(&self, mut ctx: JobContext) -> JobResult {
        let command = self.command.clone();
        let shell = self.shell.clone();
        let mut manager = self.win_manager.clone();

        let fut = async move {
            // Escape single quotes, so we can execute this command in shell
            // using single quotes
            let escaped_command = escape_cmd(&command);

            let exists = manager.has_linked_window(&shell);
            log::info!("Hash linked window: {exists}");
            if exists {
                manager.reset_linked_window(&shell)?;
                manager.run(&shell, &escaped_command)?;
            } else {
                manager.create_linked_window(&shell)?;
                manager.run(&shell, &escaped_command)?;
                ctx.send(manager);
            }

            Ok(())
        };

        Box::pin(fut)
    }
}

impl KeepInTouch for ShellCommand {
    fn client_id(&self) -> ClientId {
        self.client_id
    }

    fn on_message(&self, editor: &mut crate::editor::Editor, msg: Box<dyn std::any::Any>) {
        if let Ok(mgr) = msg.downcast::<WindowManager>() {
            let (win, _buf) = editor.win_buf_mut(self.client_id);
            win.window_manager = *mgr;
        }
    }
}
