use std::time::Duration;

use sanedit_buffer::ReadOnlyPieceTree;

use crate::{
    editor::{jobs::Job, Editor},
    server::{ClientId, JobFutureFn, JobId},
};

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
        Box::new(move |_send| Box::pin(log_buffer(ropt)))
    };
    let jobs = &mut editor.jobs;
    let job = Job::new(id, fun, None, None);
    let id = job.id();
    jobs.run(job);
    id
}
