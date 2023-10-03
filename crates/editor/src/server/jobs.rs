mod context;
mod events;
mod id;

use core::fmt::{self, Debug};
use std::{
    any::Any,
    collections::HashMap,
    fmt::Display,
    pin::Pin,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use futures::{future::BoxFuture, Future};
use parking_lot::Mutex;
use tokio::{
    sync::mpsc::{self, channel, Sender},
    task::JoinHandle,
};

use crate::{editor::Editor, events::ToEditor};

use super::{ClientId, EditorHandle, CHANNEL_SIZE};
pub(crate) use context::*;
pub(crate) use events::*;
pub(crate) use id::*;

/// Used to communicate with jobs runner
pub(crate) type JobsHandle = mpsc::Sender<ToJobs>;
/// A job that can be sent to other threads
pub(crate) type BoxedJob = Box<dyn Job + Send + Sync>;
pub(crate) type JobResult = BoxFuture<'static, Result<String, ()>>;

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
pub(crate) async fn spawn_jobs(editor_handle: EditorHandle) -> JobsHandle {
    let (tx, rx) = mpsc::channel(CHANNEL_SIZE);
    tokio::spawn(jobs_loop(rx, editor_handle));
    tx
}

// Runs jobs in tokio runtime.
async fn jobs_loop(mut recv: mpsc::Receiver<ToJobs>, handle: EditorHandle) {
    let (tx, mut rx) = channel(CHANNEL_SIZE);
    let context = JobContext {
        editor: handle,
        internal: tx,
    };
    let mut jobs = HashMap::new();

    tokio::select!(
        Some(msg) = rx.recv() => {
            use InternalJobsMessage::*;
            let _ = match msg {
                Succesful(id) => jobs.remove(&id),
                Failed(id) => jobs.remove(&id),
            };
        },
        Some(msg) = recv.recv() => {
            use ToJobs::*;
            match msg {
                Request(id, job) => {
                    let mut ctx = context.clone();
                    let task = async move {
                        let _ = match job.run(ctx.clone()).await {
                            Ok(_) => ctx.success(id).await,
                            Err(_) => ctx.failure(id).await,
                        };
                    };

                    let join = tokio::spawn(task);
                    jobs.insert(id, join);
                }
                Stop(id) => {
                    if let Some(join) = jobs.remove(&id) {
                        join.abort();
                    }
                }
            }
        },
    );
}
