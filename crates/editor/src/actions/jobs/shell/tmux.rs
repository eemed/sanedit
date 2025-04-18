use std::process::Command;

use anyhow::bail;
use sanedit_buffer::PieceTreeView;

use crate::editor::{job_broker::KeepInTouch, windows::WindowManager};
use sanedit_server::{ClientId, Job, JobContext, JobResult};

#[derive(Debug, Clone)]
pub(crate) struct TmuxPane {
    _session: usize,
    _window: usize,
    pane: usize,
}

#[derive(Clone)]
pub(crate) struct TmuxShellCommand {
    client_id: ClientId,
    command: String,
    shell: String,
    pane: Option<TmuxPane>,

    pipe_input: Option<PieceTreeView>,
}

impl TmuxShellCommand {
    pub fn new(id: ClientId, shell: &str, command: &str) -> TmuxShellCommand {
        TmuxShellCommand {
            client_id: id,
            command: command.into(),
            pipe_input: None,
            shell: shell.into(),
            pane: None,
        }
    }

    pub fn pane(mut self, pane: TmuxPane) -> Self {
        self.pane = Some(pane);
        self
    }

    #[allow(dead_code)]
    pub fn pipe(mut self, ropt: PieceTreeView) -> Self {
        self.pipe_input = Some(ropt);
        self
    }
}

impl Job for TmuxShellCommand {
    fn run(&self, mut ctx: JobContext) -> JobResult {
        let command = self.command.clone();
        let shell = self.shell.clone();
        let pane = self.pane.clone();

        let fut = async move {
            // Escape single quotes, so we can execute this command in shell
            // using single quotes
            let escaped_command = escape_cmd(&command);

            let exists = pane
                .as_ref()
                .map(|pane| pane_exists(&pane, &shell))
                .unwrap_or(false);
            if exists {
                let pane = pane.unwrap();
                reset_pane(&pane, &shell)?;
                run_in_pane(&pane, &shell, &escaped_command)?;
            } else {
                let cwin = open_pane(&shell)?;
                run_in_pane(&cwin, &shell, &escaped_command)?;
                ctx.send(cwin);
            }

            Ok(())
        };

        Box::pin(fut)
    }
}

impl KeepInTouch for TmuxShellCommand {
    fn client_id(&self) -> ClientId {
        self.client_id
    }

    fn on_message(&self, editor: &mut crate::editor::Editor, msg: Box<dyn std::any::Any>) {
        if let Ok(pane) = msg.downcast::<TmuxPane>() {
            let (win, _buf) = editor.win_buf_mut(self.client_id);
            match &mut win.window_manager {
                WindowManager::Tmux { shell_pane } => *shell_pane = Some(*pane),
                _ => {}
            }
        }
    }
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

fn run_in_pane(win: &TmuxPane, shell: &str, cmd: &str) -> anyhow::Result<()> {
    Command::new(shell)
        .args([
            "-c",
            &format!("tmux send-keys -t %{} '{cmd}' Enter", win.pane),
        ])
        .output()?;
    Ok(())
}

fn pane_exists(win: &TmuxPane, shell: &str) -> bool {
    let output = Command::new(shell)
        .args([
            "-c",
            &format!(
                "tmux display-message -pt %{} '#{{window_active}}'",
                win.pane
            ),
        ])
        .output();

    if output.is_err() {
        return false;
    }
    let output = output.unwrap();
    let string = std::str::from_utf8(&output.stdout);

    if string.is_err() {
        return false;
    }

    string.unwrap().trim() == "1"
}

fn reset_pane(win: &TmuxPane, shell: &str) -> anyhow::Result<()> {
    Command::new(shell)
        .args([
            "-c",
            &format!("tmux respawn-pane -k -t %{} '{shell}'", win.pane),
        ])
        .output()?;
    Ok(())
}

fn open_pane(shell: &str) -> anyhow::Result<TmuxPane> {
    let tmux_cmd = format!("tmux split-window -l 15% -d -P -F \"#{{session_id}}\n#{{window_id}}\n#{{pane_id}}\" '{shell}'");
    let output = Command::new(shell).args(["-c", &tmux_cmd]).output()?;

    let output = std::str::from_utf8(&output.stdout)?.trim();
    let ids: Vec<&str> = output.split('\n').collect();
    if ids.len() != 3 {
        bail!("Command output invalid.");
    }

    let session: usize = ids[0][1..].parse()?;
    let window: usize = ids[1][1..].parse()?;
    let pane: usize = ids[2][1..].parse()?;

    Ok(TmuxPane {
        _session: session,
        _window: window,
        pane,
    })
}
