mod context;
mod events;
mod id;
mod kill;

use crossbeam::channel::Sender;
pub use futures_core::future::BoxFuture;
use rustc_hash::FxHashMap;
use tokio::sync::mpsc::{self, channel};
use tokio::task::JoinHandle;

use crate::events::ToEditor;
use crate::CHANNEL_SIZE;

pub use context::*;
pub use events::*;
pub use id::*;
pub use kill::Kill;

/// Used to communicate with jobs runner
pub type JobsHandle = mpsc::Sender<ToJobs>;
/// A job that can be sent to other threads
pub type BoxedJob = Box<dyn Job + Send + Sync>;
pub type JobResult = BoxFuture<'static, anyhow::Result<()>>;

/// Jobs that can be ran on async runner
pub trait Job {
    /// Run the job.
    /// This should return the async future to run the job.
    /// This should not block for a long time
    fn run(&self, ctx: JobContext) -> JobResult;
}

/// Jobs that do not need / would block the async runtime are ran on a separate threadpool.
pub trait CPUJob: Clone + Send + Sync {
    fn run(&self, ctx: JobContext) -> anyhow::Result<()>;
}

impl<T: CPUJob + 'static> Job for T {
    fn run(&self, ctx: JobContext) -> JobResult {
        let job = self.clone();

        let fut = async move {
            // Send result in a oneshot channel
            let (send, recv) = tokio::sync::oneshot::channel();
            rayon::spawn(move || {
                let result = job.run(ctx);
                let _ = send.send(result);
            });
            recv.await?
        };

        Box::pin(fut)
    }
}

/// Spawn a job runner
pub async fn spawn_job_runner(sender: Sender<ToEditor>) -> JobsHandle {
    let (tx, rx) = mpsc::channel(CHANNEL_SIZE);
    tokio::spawn(jobs_loop(rx, sender));
    tx
}

#[derive(Debug, Default)]
struct Jobs(FxHashMap<JobId, (Kill, JoinHandle<()>)>);

impl std::ops::Deref for Jobs {
    type Target = FxHashMap<JobId, (Kill, JoinHandle<()>)>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for Jobs {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Drop for Jobs {
    fn drop(&mut self) {
        for (_id, (kill, join)) in &self.0 {
            kill.stop();
            join.abort();
        }
    }
}

// Runs jobs in tokio runtime.
async fn jobs_loop(mut recv: mpsc::Receiver<ToJobs>, sender: crossbeam::channel::Sender<ToEditor>) {
    let (tx, mut rx) = channel(CHANNEL_SIZE);
    let context = JobResponseSender {
        editor: sender,
        internal: tx,
    };
    let mut jobs = Jobs::default();

    loop {
        tokio::select!(
            Some(msg) = rx.recv() => {
                let id = msg.id();
                jobs.remove(&id);
                let _ = context.editor.send(ToEditor::Jobs(msg.into()));
            },
            Some(msg) = recv.recv() => {
                use ToJobs::*;
                match msg {
                    Request(id, job) => {
                        let mut ictx = context.clone();
                        let ctx = ictx.to_job_context(id);
                        let kill = ctx.kill.clone();
                        let task = async move {
                            let result = job.run(ctx).await;
                            let _ = match result {
                                Ok(_) => ictx.success(id).await,
                                Err(reason) => ictx.failure(id, reason.to_string()).await,
                            };
                        };

                        let join = tokio::spawn(task);
                        jobs.insert(id, (kill, join));
                    }
                    Stop(id) => {
                        if let Some((kill, join)) = jobs.remove(&id) {
                            kill.stop();
                            join.abort();
                            let _ = join.await;
                            let _ = context.editor.send(ToEditor::Jobs(FromJobs::Stopped(id)));
                        }
                    }
                }
            },
            else => break,
        );
    }
}
