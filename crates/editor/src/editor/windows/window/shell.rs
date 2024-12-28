use crate::actions::jobs::TmuxPane;

/// How to execute shell commands
#[derive(Debug)]
pub(crate) enum ShellKind {
    Tmux { pane: Option<TmuxPane> },
}

impl Default for ShellKind {
    fn default() -> Self {
        // TODO determine
        ShellKind::Tmux { pane: None }
    }
}
