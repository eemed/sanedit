use std::{any::Any, mem, path::PathBuf, sync::Arc};

use tokio::{fs, io};

use crate::{
    actions::prompt,
    editor::{jobs::Job, Editor},
    server::{ClientId, JobFutureFn, JobId, JobProgress, JobProgressSender},
};

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
                        send.send(JobProgress::Output(Box::new(mem::take(&mut entries))))
                            .await;
                    }
                }
            }
        }

        if !entries.is_empty() {
            send.send(JobProgress::Output(Box::new(entries))).await;
        }

        Ok(())
    }

    read_recursive(send, dir).await.is_ok()
}

pub(crate) fn list_files_provide_completions(editor: &mut Editor, id: ClientId) -> JobId {
    let fun: JobFutureFn = {
        let cwd = editor.working_dir().to_path_buf();
        Box::new(move |send| Box::pin(list_files(send, cwd)))
    };
    let jobs = &mut editor.jobs;
    let on_output = Arc::new(|editor: &mut Editor, id: ClientId, out: Box<dyn Any>| {
        if let Ok(output) = out.downcast::<Vec<String>>() {
            prompt::provide_completions(editor, id, *output);
        }
    });
    let job = Job::new(id, fun).on_output(on_output);
    jobs.request(job)
}
