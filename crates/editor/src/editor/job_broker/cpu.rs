use std::any::Any;

use tokio::sync::oneshot::channel;

use crate::{
    editor::{job_broker::KeepInTouch, Editor},
    job_runner::{Job, JobContext, JobResult},
    server::ClientId,
};

/// Jobs that do not need / would block the async runtime are ran on a separate threadpool.
pub(crate) trait CPUJob: Clone + Send + Sync {
    fn run(&self, ctx: JobContext) -> anyhow::Result<()>;
}

#[derive(Clone, Debug)]
pub(crate) struct CPU<T: CPUJob + 'static> {
    job: T,
}

impl<T: CPUJob> CPU<T> {
    pub fn new(job: T) -> CPU<T> {
        CPU { job }
    }
}

impl<T: CPUJob> Job for CPU<T> {
    fn run(&self, ctx: JobContext) -> JobResult {
        let job = self.job.clone();

        let fut = async move {
            // Send result in a oneshot channel
            let (send, recv) = channel();
            rayon::spawn(move || {
                let result = job.run(ctx);
                let _ = send.send(result);
            });
            recv.await?
        };

        Box::pin(fut)
    }
}

impl<T: CPUJob + KeepInTouch> KeepInTouch for CPU<T> {
    fn client_id(&self) -> ClientId {
        self.job.client_id()
    }

    fn on_message(&self, editor: &mut Editor, msg: Box<dyn Any>) {
        self.job.on_message(editor, msg);
    }

    fn on_success(&self, editor: &mut Editor) {
        self.job.on_success(editor);
    }

    fn on_failure(&self, editor: &mut Editor, reason: &str) {
        self.job.on_failure(editor, reason);
    }
}
