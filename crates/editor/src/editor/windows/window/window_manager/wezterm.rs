use std::process::Command;

#[derive(Debug, Clone, Default)]
pub(crate) struct Wezterm {
    pane: Option<WeztermPane>,
}

impl Wezterm {
    pub fn new_window_horizontal(&self, cmd: &str) -> String {
        format!("wezterm cli split-pane -- {cmd}")
    }

    pub fn new_window_vertical(&self, cmd: &str) -> String {
        format!("wezterm cli split-pane --horizontal -- {cmd}")
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
        let current_pane = std::env::var("WEZTERM_PANE")?;

        let cmd = format!("wezterm cli split-pane --percent 15 -- {shell}");
        let output = Command::new(shell).args(["-c", &cmd]).output()?;

        let output = std::str::from_utf8(&output.stdout)?.trim();
        let pane: usize = output.parse()?;
        self.pane = Some(WeztermPane { pane });

        // Refocus editor
        let cmd = format!("wezterm cli activate-pane --pane-id {current_pane}");
        Command::new(shell).args(["-c", &cmd]).output()?;

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
pub(crate) struct WeztermPane {
    pane: usize,
}

fn cmd_ok(cmd: &mut Command) -> bool {
    match cmd.status() {
        Ok(status) => status.success(),
        Err(_) => false,
    }
}

impl WeztermPane {
    fn exists(&self, shell: &str) -> bool {
        let Ok(current_pane) = std::env::var("WEZTERM_PANE") else {
            return false;
        };
        let cmd = format!("wezterm cli activate-pane --pane-id {}", self.pane);
        if !cmd_ok(Command::new(shell).args(["-c", &cmd])) {
            return false;
        }

        let cmd = format!("wezterm cli activate-pane --pane-id {current_pane}");
        if !cmd_ok(Command::new(shell).args(["-c", &cmd])) {
            return false;
        }

        true
    }

    fn reset(&self, _shell: &str) -> anyhow::Result<()> {
        // TODO not supported
        Ok(())
    }

    fn run(&self, shell: &str, cmd: &str) -> anyhow::Result<()> {
        let cmd = format!(
            "wezterm cli send-text --no-paste --pane-id {} '{cmd}\n'",
            self.pane
        );
        Command::new(shell).args(["-c", &cmd]).output()?;
        Ok(())
    }
}
