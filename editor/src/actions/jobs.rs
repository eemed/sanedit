use std::{pin::Pin, time::Duration};

use futures::Future;

use crate::{
    editor::Editor,
    server::{ClientId, EditorHandle, Job},
};

type PinnedFuture = Pin<Box<dyn Future<Output = bool> + Send + Sync>>;

fn list_files(handle: EditorHandle) -> PinnedFuture {
    Box::pin(async {
        log::info!("list_files hello");
        tokio::time::sleep(Duration::from_secs(2)).await;
        true
    })
}

pub(crate) fn jobs_test(editor: &mut Editor, id: ClientId) {
    let jobs = editor.jobs_mut();
    let boxed = Box::new(list_files);
    let job = Job::new(boxed);
    jobs.new_job(job);
}
