use std::process::Command;

use super::TerminalWindowManager;

#[derive(Debug, Clone, Default)]
pub(crate) struct Wezterm {}

impl TerminalWindowManager for Wezterm {
    fn new_window_horizontal(&mut self, shell: &str, new_instance_cmd: &str) {
        let split = format!("wezterm cli split-pane -- {new_instance_cmd} && {shell}");
        let _ = Command::new(shell).args(["-c", &split]).output();
    }

    fn new_window_vertical(&mut self, shell: &str, new_instance_cmd: &str) {
        let split = format!("wezterm cli split-pane --horizontal -- {new_instance_cmd} && {shell}");
        let _ = Command::new(shell).args(["-c", &split]).output();
    }
}
