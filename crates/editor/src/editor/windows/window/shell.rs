use crate::actions::jobs::TmuxPane;

/// How to execute shell commands
#[derive(Debug)]
pub(crate) enum ShellKind {
    Tmux { pane: Option<TmuxPane> },
    NonInteractive,
}

impl Default for ShellKind {
    fn default() -> Self {
        let is_tmux = std::env::var("TMUX")
            .map(|var| !var.is_empty())
            .unwrap_or(false);
        if is_tmux {
            return ShellKind::Tmux { pane: None };
        }

        ShellKind::NonInteractive
    }
}
