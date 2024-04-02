use tokio::sync::oneshot::channel;

use crate::job_runner::{Job, JobContext, JobResult};

/// Jobs that do not need / would block the async runtime are ran on a separate threadpool.
pub(crate) trait CPUJob: Clone + Send + Sync {
    fn run(&self, ctx: JobContext) -> anyhow::Result<()>;
}

impl<T: CPUJob + 'static> Job for T {
    fn run(&self, ctx: JobContext) -> JobResult {
        let job = self.clone();

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
