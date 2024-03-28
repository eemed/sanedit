use std::{fs, path::PathBuf};

use sanedit_buffer::ReadOnlyPieceTree;
use tokio::{fs::File, io::AsyncWriteExt};

use crate::{
    editor::{job_broker::KeepInTouch, Editor},
    job_runner::{BoxedJob, Job, JobContext, JobResult},
    server::ClientId,
};

#[derive(Clone)]
pub(crate) struct Save {
    client_id: ClientId,
    buf: ReadOnlyPieceTree,
    to: PathBuf,
}

impl Save {
    pub fn new(id: ClientId, buf: ReadOnlyPieceTree, to: PathBuf) -> Save {
        Save {
            client_id: id,
            buf,
            to,
        }
    }
}

impl Job for Save {
    fn run(&self, mut ctx: JobContext) -> JobResult {
        let buf = self.buf.clone();
        let to = self.to.clone();

        let fut = async move {
            let mut file = File::create(&to).await?;

            let mut chunks = buf.chunks();
            let mut chunk = chunks.get();
            while let Some((_, chk)) = chunk {
                let bytes = chk.as_ref();
                file.write(bytes).await?;
                chunk = chunks.next();
            }

            file.flush().await?;
            Ok(())
        };

        Box::pin(fut)
    }

    fn box_clone(&self) -> BoxedJob {
        Box::new((*self).clone())
    }
}

impl KeepInTouch for Save {
    fn client_id(&self) -> ClientId {
        self.client_id
    }

    fn on_success(&self, editor: &mut Editor) {
        let (_win, buf) = editor.win_buf_mut(self.client_id);
        if let Err(e) = buf.save_succesful(&self.to) {
            log::error!("Failed to save file to {:?}: {e}", self.to);
            // cleanup file
            let _ = fs::remove_file(&self.to);
        }
    }

    fn on_failure(&self, editor: &mut Editor, reason: &str) {
        let (_win, buf) = editor.win_buf_mut(self.client_id);
        buf.save_failed();
    }
}
