mod tmux;
mod wezterm;
use tmux::Tmux;
use wezterm::Wezterm;

#[derive(Debug)]
pub(crate) struct WindowManager {
    shell: String,
    new_instance_command: String,
    wm: Box<dyn TerminalWindowManager>,
}

impl WindowManager {
    pub fn new(window_id: usize, session: &str) -> WindowManager {
        let shell = "/bin/bash".to_string();
        let new_instance_command = format!("sane --session {session} --parent-client {window_id}");
        let wm = Self::new_wm();

        WindowManager {
            shell,
            new_instance_command,
            wm,
        }
    }

    fn new_wm() -> Box<dyn TerminalWindowManager> {
        if let Ok(prog) = std::env::var("TERM_PROGRAM") {
            return match prog.as_str() {
                "WezTerm" => Box::new(Wezterm::default()),
                "tmux" => Box::new(Tmux::default()),
                _ => Box::new(Empty),
            };
        }

        if std::env::var("TMUX").is_ok() {
            return Box::new(Tmux::default());
        }

        if std::env::var("WEZTERM_PANE").is_ok() {
            return Box::new(Wezterm::default());
        }

        Box::new(Empty)
    }

    pub fn new_window_horizontal(&mut self) {
        self.wm.new_window_horizontal(&self.shell, &self.new_instance_command);
    }

    pub fn new_window_vertical(&mut self) {
        self.wm.new_window_vertical(&self.shell, &self.new_instance_command);
    }
}

pub(crate) trait TerminalWindowManager: std::fmt::Debug {
    fn new_window_horizontal(&mut self, shell: &str, new_instance_cmd: &str);
    fn new_window_vertical(&mut self, shell: &str, new_instance_cmd: &str);
}

#[derive(Debug)]
struct Empty;

impl TerminalWindowManager for Empty {
    fn new_window_horizontal(&mut self, _shell: &str, _new_instance_cmd: &str) {}
    fn new_window_vertical(&mut self, _shell: &str, _new_instance_cmd: &str) {}
}
