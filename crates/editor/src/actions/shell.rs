use crate::{
    editor::{windows::Executor, Editor},
    server::ClientId,
};

use super::jobs::TmuxShellCommand;

pub(crate) fn execute(editor: &mut Editor, id: ClientId, cmd: &str) {
    let (win, _buf) = editor.win_buf(id);
    match &win.cmds.executor {
        Executor::Tmux { pane } => {
            let mut job = TmuxShellCommand::new(id, &win.cmds.shell, cmd);
            if let Some(pane) = pane {
                job = job.pane(pane.clone());
            }

            editor.job_broker.request(job);
        }
        Executor::Buffer => unimplemented!("In buffer commands"),
    }
}
