use crate::actions::jobs::TmuxPane;

#[derive(Debug)]
pub(crate) enum WindowManager {
    Tmux { shell_pane: Option<TmuxPane> },
}
