use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use tokio::fs;

use crate::{
    editor::Editor,
    server::{ClientId, Job, JobFutureFn, JobProgress, JobProgressSender, PinnedFuture},
};

async fn list_files(mut send: JobProgressSender, dir: PathBuf) -> bool {
    log::info!("list_files hello");
    send.send(JobProgress::Output(vec![
        "hello".to_string(),
        "world".to_string(),
    ]))
    .await;
    tokio::time::sleep(Duration::from_secs(2)).await;

    // match fs::read_dir(&cwd).await {
    //     Ok(_) => {}
    //     Err(_) => {}
    // }

    true
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
