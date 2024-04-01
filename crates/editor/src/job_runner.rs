mod context;
mod events;
mod id;

use std::collections::HashMap;

use futures::future::BoxFuture;
use tokio::sync::mpsc::{self, channel};

use crate::events::ToEditor;

use super::{EditorHandle, CHANNEL_SIZE};
pub(crate) use context::*;
pub(crate) use events::*;
pub(crate) use id::*;

/// Used to communicate with jobs runner
pub(crate) type JobsHandle = mpsc::Sender<ToJobs>;
/// A job that can be sent to other threads
pub(crate) type BoxedJob = Box<dyn Job + Send + Sync>;
pub(crate) type JobResult = BoxFuture<'static, anyhow::Result<()>>;

/// Jobs that can be ran on async runner
pub(crate) trait Job {
    /// Run the job.
    /// This should return the async future to run the job.
    /// This should not block for a long time
    fn run(&self, ctx: JobContext) -> JobResult;

    /// Clone the job and transform it into sendable form
    fn box_clone(&self) -> BoxedJob;
}

/// Spawn a job runner
pub(crate) async fn spawn_job_runner(editor_handle: EditorHandle) -> JobsHandle {
    let (tx, rx) = mpsc::channel(CHANNEL_SIZE);
    tokio::spawn(jobs_loop(rx, editor_handle));
    tx
}

// Runs jobs in tokio runtime.
async fn jobs_loop(mut recv: mpsc::Receiver<ToJobs>, handle: EditorHandle) {
    let (tx, mut rx) = channel(CHANNEL_SIZE);
    let mut context = JobResponseSender {
        editor: handle,
        internal: tx,
    };
    let mut jobs = HashMap::new();

    loop {
        tokio::select!(
            Some(msg) = rx.recv() => {
                let id = msg.id();
                jobs.remove(&id);
                context.editor.send(ToEditor::Jobs(msg.into()));
            },
            Some(msg) = recv.recv() => {
                use ToJobs::*;
                match msg {
                    Request(id, job) => {
                        let mut ictx = context.clone();
                        let (kill, ctx) = ictx.to_job_context(id);
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
                            let _ = kill.send(());
                            join.abort();
                            let _ = join.await;
                            log::info!("Job {id} stopped");
                        }
                    }
                }
            },
            else => break,
        );
    }
}
