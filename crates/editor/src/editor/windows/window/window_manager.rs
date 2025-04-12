use crate::actions::jobs::TmuxPane;

#[derive(Debug)]
pub(crate) enum WindowManager {
    Tmux { shell_pane: Option<TmuxPane> },
    Wezterm,
}

impl WindowManager {
    pub fn new_window(&self) -> String {
        match self {
            WindowManager::Tmux { .. } => "tmux split-window 'sane'".into(),
            WindowManager::Wezterm { .. } => "wezterm cli split-pane -- sane".into(),
        }
    }
}
