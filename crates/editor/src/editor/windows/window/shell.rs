use crate::actions::jobs::TmuxPane;

/// How to execute shell commands
#[derive(Debug)]
pub(crate) enum Executor {
    Tmux { pane: Option<TmuxPane> },
}

impl Default for Executor {
    fn default() -> Self {
        // TODO determine
        Executor::Tmux { pane: None }
    }
}
