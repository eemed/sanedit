use std::{fs, path::PathBuf};

use sanedit_buffer::ReadOnlyPieceTree;
use tokio::{fs::File, io::AsyncWriteExt};

use crate::{
    common::dirs::tmp_file,
    editor::{job_broker::KeepInTouch, Editor},
    job_runner::{Job, JobContext, JobResult},
    server::ClientId,
};

#[derive(Clone)]
pub(crate) struct Save {
    client_id: ClientId,
    buf: ReadOnlyPieceTree,
}

impl Save {
    pub fn new(id: ClientId, buf: ReadOnlyPieceTree) -> Save {
        Save { client_id: id, buf }
    }

    pub async fn save(buf: ReadOnlyPieceTree, file: std::fs::File) -> anyhow::Result<()> {
        let mut file = File::from_std(file);

        let mut chunks = buf.chunks();
        let mut chunk = chunks.get();
        while let Some((_, chk)) = chunk {
            let bytes = chk.as_ref();
            file.write(bytes).await?;
            chunk = chunks.next();
        }

        file.flush().await?;
        Ok(())
    }
}

impl Job for Save {
    fn run(&self, mut ctx: JobContext) -> JobResult {
        let buf = self.buf.clone();

        let fut = async move {
            let (path, file) = tmp_file().ok_or(anyhow::anyhow!("Cannot create tempfile"))?;
            let ok = Self::save(buf, file).await.is_ok();
            ctx.send(Msg { ok, path });

            Ok(())
        };

        Box::pin(fut)
    }
}

struct Msg {
    ok: bool,
    path: PathBuf,
}

impl KeepInTouch for Save {
    fn client_id(&self) -> ClientId {
        self.client_id
    }

    fn on_failure(&self, editor: &mut Editor, reason: &str) {
        let (_win, buf) = editor.win_buf_mut(self.client_id);
        buf.read_only = false;
    }

    fn on_success(&self, editor: &mut Editor) {
        let (_win, buf) = editor.win_buf_mut(self.client_id);
        buf.read_only = false;
    }

    fn on_message(&self, editor: &mut Editor, msg: Box<dyn std::any::Any>) {
        let (_win, buf) = editor.win_buf_mut(self.client_id);

        if let Ok(msg) = msg.downcast::<Msg>() {
            if msg.ok {
                buf.async_save_succesful(&msg.path);
            } else {
                buf.async_save_failed();
            }

            let _ = fs::remove_file(&msg.path);
        }
    }
}
