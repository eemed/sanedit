use std::process::Command;

use super::TerminalWindowManager;

#[derive(Debug, Clone, Default)]
pub(crate) struct Tmux {}

impl TerminalWindowManager for Tmux {
    fn new_window_horizontal(&mut self, shell: &str, new_instance_cmd: &str) {
        let split = format!("tmux split-window {shell} -c '{new_instance_cmd}; exec {shell}'");
        let _ = Command::new(shell).args(["-c", &split]).output();
    }

    fn new_window_vertical(&mut self, shell: &str, new_instance_cmd: &str) {
        let split = format!("tmux split-window -h {shell} -c '{new_instance_cmd}; exec {shell}'");
        let _ = Command::new(shell).args(["-c", &split]).output();
    }
}
