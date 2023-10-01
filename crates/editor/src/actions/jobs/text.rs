use std::{any::Any, io, path::PathBuf, sync::Arc};

use sanedit_buffer::ReadOnlyPieceTree;
use tokio::{fs::File, io::AsyncWriteExt};

use crate::{
    common::dirs::tmp_dir,
    editor::{jobs::Job, Editor},
    server::{ClientId, JobFutureFn, JobProgress, JobProgressSender},
};

pub(crate) fn save_file(editor: &mut Editor, id: ClientId) -> io::Result<Job> {
    let target = {
        let (_, buf) = editor.win_buf_mut(id);
        let name: &str = &buf.name();
        let mut tmp = tmp_dir().ok_or(io::Error::new(
            io::ErrorKind::NotFound,
            "temporary directory not found",
        ))?;

        tmp.push(PathBuf::from(name));
        tmp
    };

    let fun: JobFutureFn = {
        let (_, buf) = editor.win_buf_mut(id);
        let ropt = buf.read_only_copy();
        buf.start_saving();
        Box::new(move |send| Box::pin(save(send, ropt, target)))
    };

    let on_output = Arc::new(|editor: &mut Editor, id: ClientId, out: Box<dyn Any>| {
        let (win, buf) = editor.win_buf_mut(id);
        if let Ok(tmp) = out.downcast::<PathBuf>() {
            // TODO: if buf was file backed, swap the files around so original
            // is in tmp and use that as backing file instead. this needs to be
            // done 1 time only. After that the saving is done normally
            //
            // buf.save_rename_file_backed() ?? that does this or just do it in
            // save_rename
            match buf.save_rename(&*tmp) {
                Ok(_) => win.info_msg(&format!("Buffer {} saved", buf.name())),
                Err(e) => win.error_msg(&format!("Failed to save buffer {}, {e:?}", buf.name())),
            }
        }
    });

    let on_error = Arc::new(|editor: &mut Editor, id: ClientId, out: Box<dyn Any>| {
        let (win, buf) = editor.win_buf_mut(id);
        if let Ok(e) = out.downcast::<String>() {
            buf.save_failed();
            let msg = match buf.path() {
                Some(fpath) => format!("Error while renaming file {fpath:?}, {e:?}"),
                None => format!("Path not set for buffer {}", buf.name()),
            };
            win.error_msg(&msg);
        }
    });

    Ok(Job::new(id, fun).on_output(on_output).on_error(on_error))
}

async fn save(mut send: JobProgressSender, ropt: ReadOnlyPieceTree, to: PathBuf) -> bool {
    async fn save_impl(ropt: ReadOnlyPieceTree, to: PathBuf) -> Result<(), tokio::io::Error> {
        let mut file = File::create(&to).await?;

        let mut chunks = ropt.chunks();
        let mut chunk = chunks.get();
        while let Some((_, chk)) = chunk {
            let bytes = chk.as_ref();
            file.write(bytes).await?;
            chunk = chunks.next();
        }

        file.flush().await?;
        Ok(())
    }

    use JobProgress::*;
    let msg = match save_impl(ropt, to.clone()).await {
        Ok(_) => Output(Box::new(to)),
        Err(e) => Error(Box::new(e.to_string())),
    };
    send.send(msg).await.is_ok()
}
