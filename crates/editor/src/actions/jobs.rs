use std::{mem, path::PathBuf, sync::Arc, time::Duration};

use sanedit_buffer::ReadOnlyPieceTree;
use tokio::{fs, io};

use crate::{
    editor::{jobs::AsyncJob, Editor},
    server::{ClientId, JobFutureFn, JobId, JobProgress, JobProgressSender},
};

use super::prompt;

async fn list_files(send: JobProgressSender, dir: PathBuf) -> bool {
    async fn read_recursive(mut send: JobProgressSender, base: PathBuf) -> io::Result<()> {
        let mut entries: Vec<String> = Vec::new();
        let mut stack: Vec<PathBuf> = Vec::new();
        stack.push(base.clone());

        while let Some(dir) = stack.pop() {
            let mut read_dir = fs::read_dir(&dir).await?;
            while let Ok(Some(entry)) = read_dir.next_entry().await {
                let path = entry.path();
                let metadata = entry.metadata().await?;
                if metadata.is_dir() {
                    stack.push(path);
                } else {
                    let stripped = path.strip_prefix(&base).unwrap();
                    let name: String = stripped.to_string_lossy().into();
                    entries.push(name);

                    if entries.len() > 2000 {
                        send.send(JobProgress::Output(mem::take(&mut entries)))
                            .await;
                    }
                }
            }
        }

        if !entries.is_empty() {
            send.send(JobProgress::Output(entries)).await;
        }

        Ok(())
    }

    log::info!("List files");
    read_recursive(send, dir).await.is_ok()
}

pub(crate) fn list_files_provide_completions(editor: &mut Editor, id: ClientId) -> JobId {
    let fun: JobFutureFn = {
        let cwd = editor.working_dir().to_path_buf();
        Box::new(move |send| Box::pin(list_files(send, cwd)))
    };
    let jobs = &mut editor.jobs;
    let on_output = Arc::new(prompt::provide_completions);
    let job = AsyncJob::new(id, fun, Some(on_output), None);
    let id = job.id();
    jobs.run(job);
    id
}

async fn log_buffer(ropt: ReadOnlyPieceTree) -> bool {
    tokio::time::sleep(Duration::from_secs(2)).await;

    let slice = ropt.slice(..);
    let string = String::from(&slice);
    log::info!("Read buffer {}", string);
    true
}

pub(crate) fn log_buffer_after_timeout(editor: &mut Editor, id: ClientId) -> JobId {
    let fun: JobFutureFn = {
        let (_, buf) = editor.win_buf(id);
        let ropt = buf.read_only_copy();
        Box::new(move |send| Box::pin(log_buffer(ropt)))
    };
    let jobs = &mut editor.jobs;
    let job = AsyncJob::new(id, fun, None, None);
    let id = job.id();
    jobs.run(job);
    id
}
