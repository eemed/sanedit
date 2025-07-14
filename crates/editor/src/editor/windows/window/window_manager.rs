mod tmux;
mod wezterm;
use anyhow::bail;
use tmux::Tmux;
use wezterm::Wezterm;

#[derive(Debug, Clone)]
pub(crate) enum WindowManager {
    Tmux(Tmux),
    Wezterm(Wezterm),
    None,
}

impl WindowManager {
    pub fn tmux() -> WindowManager {
        WindowManager::Tmux(Tmux::default())
    }

    pub fn wezterm() -> WindowManager {
        WindowManager::Wezterm(Wezterm::default())
    }

    pub fn new_window_horizontal(&self) -> String {
        match self {
            WindowManager::Tmux(tmux) => tmux.new_window_horizontal(),
            WindowManager::Wezterm(wezterm) => wezterm.new_window_horizontal(),
            WindowManager::None => "".into(),
        }
    }

    pub fn new_window_vertical(&self) -> String {
        match self {
            WindowManager::Tmux(tmux) => tmux.new_window_vertical(),
            WindowManager::Wezterm(wezterm) => wezterm.new_window_vertical(),
            WindowManager::None => "".into(),
        }
    }

    pub fn has_linked_window(&self, shell: &str) -> bool {
        match self {
            WindowManager::Tmux(tmux) => tmux.has_linked_window(shell),
            WindowManager::Wezterm(wezterm) => wezterm.has_linked_window(shell),
            WindowManager::None => false,
        }
    }

    pub fn reset_linked_window(&mut self, shell: &str) -> anyhow::Result<()> {
        match self {
            WindowManager::Tmux(tmux) => tmux.reset_linked_window(shell),
            WindowManager::Wezterm(wezterm) => wezterm.reset_linked_window(shell),
            WindowManager::None => bail!("No window manager"),
        }
    }

    pub fn create_linked_window(&mut self, shell: &str) -> anyhow::Result<()> {
        match self {
            WindowManager::Tmux(tmux) => tmux.create_linked_window(shell),
            WindowManager::Wezterm(wezterm) => wezterm.create_linked_window(shell),
            WindowManager::None => bail!("No window manager"),
        }
    }

    pub fn run(&mut self, shell: &str, cmd: &str) -> anyhow::Result<()> {
        match self {
            WindowManager::Tmux(tmux) => tmux.run(shell, cmd),
            WindowManager::Wezterm(wezterm) => wezterm.run(shell, cmd),
            WindowManager::None => bail!("No window manager"),
        }
    }
}
