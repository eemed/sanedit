use crate::actions::jobs::TmuxPane;

/// Where and how to run shell commands
#[derive(Debug)]
pub(crate) struct Commands {
    pub(crate) shell: String,
    pub(crate) executor: Executor,
}

impl Default for Commands {
    fn default() -> Self {
        Commands {
            shell: "/bin/bash".into(),
            executor: Executor::default(),
        }
    }
}

#[derive(Debug)]
pub(crate) enum Executor {
    Tmux { pane: Option<TmuxPane> },
    Buffer,
}

impl Default for Executor {
    fn default() -> Self {
        // TODO determine
        Executor::Tmux { pane: None }
    }
}
