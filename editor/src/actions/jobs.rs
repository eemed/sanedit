use std::{
    mem,
    path::{Path, PathBuf},
    time::Duration,
};

use futures::{future::LocalBoxFuture, Future, FutureExt};
use tokio::{
    fs::{self, DirEntry},
    io,
};

use crate::{
    editor::Editor,
    server::{ClientId, Job, JobFutureFn, JobProgress, JobProgressSender},
};

async fn list_files(mut send: JobProgressSender, dir: PathBuf) -> bool {
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

                    if entries.len() > 100 {
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

pub(crate) fn jobs_test(editor: &mut Editor, id: ClientId) {
    let fun: JobFutureFn = {
        let cwd = editor.working_dir().to_path_buf();
        Box::new(move |send| Box::pin(list_files(send, cwd)))
    };
    let jobs = editor.jobs_mut();
    let job = Job::new(fun);
    jobs.run_job(job);
}
