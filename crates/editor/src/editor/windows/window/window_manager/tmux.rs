use std::process::Command;

use anyhow::bail;

#[derive(Debug, Clone, Default)]
pub(crate) struct Tmux {
    pane: Option<TmuxPane>,
}

impl Tmux {
    pub fn new_window_horizontal(&self) -> String {
        "tmux split-window 'sane'".into()
    }

    pub fn new_window_vertical(&self) -> String {
        "tmux split-window -h 'sane'".into()
    }

    pub fn has_linked_window(&self, shell: &str) -> bool {
        if let Some(pane) = &self.pane {
            return pane.exists(shell);
        }

        false
    }

    pub fn reset_linked_window(&self, shell: &str) -> anyhow::Result<()> {
        if let Some(pane) = &self.pane {
            return pane.reset(shell);
        }

        Ok(())
    }

    pub fn create_linked_window(&mut self, shell: &str) -> anyhow::Result<()> {
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

        self.pane = Some(TmuxPane {
            _session: session,
            _window: window,
            pane,
        });
        Ok(())
    }

    pub fn run(&self, shell: &str, cmd: &str) -> anyhow::Result<()> {
        if let Some(pane) = &self.pane {
            return pane.run(shell, cmd);
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub(crate) struct TmuxPane {
    _session: usize,
    _window: usize,
    pane: usize,
}

impl TmuxPane {
    fn exists(&self, shell: &str) -> bool {
        let output = Command::new(shell)
            .args([
                "-c",
                &format!(
                    "tmux display-message -pt %{} '#{{window_active}}'",
                    self.pane
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

    fn reset(&self, shell: &str) -> anyhow::Result<()> {
        Command::new(shell)
            .args([
                "-c",
                &format!("tmux respawn-pane -k -t %{} '{shell}'", self.pane),
            ])
            .output()?;
        Ok(())
    }

    fn run(&self, shell: &str, cmd: &str) -> anyhow::Result<()> {
        Command::new(shell)
            .args([
                "-c",
                &format!("tmux send-keys -t %{} '{cmd}' Enter", self.pane),
            ])
            .output()?;
        Ok(())
    }
}
