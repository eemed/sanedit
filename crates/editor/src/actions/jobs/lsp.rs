use std::{ffi::OsStr, path::PathBuf};

use sanedit_lsp::LSPContext;

use crate::{
    editor::job_broker::KeepInTouch,
    job_runner::{Job, JobContext, JobResult},
    server::ClientId,
};

#[derive(Clone)]
pub(crate) struct LSP {
    client_id: ClientId,
    working_dir: PathBuf,
}

impl LSP {
    pub fn new(id: ClientId, working_dir: PathBuf) -> LSP {
        LSP {
            client_id: id,
            working_dir,
        }
    }
}

impl Job for LSP {
    fn run(&self, ctx: JobContext) -> JobResult {
        // Clones here
        let wd = self.working_dir.clone();

        let fut = async move {
            log::info!("Run rust-analyzer");
            // Implementation
            let mut ctx = LSPContext {
                run_command: "rust-analyzer".into(),
                run_args: vec![],
                root: wd,
            };

            match ctx.spawn().await {
                Ok(_) => log::error!("LSP ok"),
                Err(e) => log::error!("LSP error: {e}"),
            }
            Ok(())
        };

        Box::pin(fut)
    }
}

impl KeepInTouch for LSP {
    fn client_id(&self) -> ClientId {
        self.client_id
    }
}
