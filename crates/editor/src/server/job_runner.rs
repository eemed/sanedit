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
pub(crate) type JobResult = BoxFuture<'static, Result<(), String>>;

/// Jobs that can be ran on async runner
pub(crate) trait Job {
    /// Run the job.
    /// This should return the async future to run the job.
    /// This should not block for a long time
    fn run(&self, ctx: &JobContext) -> JobResult;

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
    let mut context = InternalJobContext {
        editor: handle,
        internal: tx,
    };
    let mut jobs = HashMap::new();

    tokio::select!(
        Some(msg) = rx.recv() => {
            let id = msg.id();
            jobs.remove(&id);
            context.editor.send(ToEditor::Jobs(msg.into())).await;
        },
        Some(msg) = recv.recv() => {
            use ToJobs::*;
            match msg {
                Request(id, job) => {
                    let mut ctx = context.to_job_context(id);
                    let task = async move {
                        let result = job.run(&ctx).await;
                        let mut ctx: InternalJobContext = ctx.into();
                        let _ = match result {
                            Ok(_) => ctx.success(id).await,
                            Err(reason) => ctx.failure(id, reason).await,
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
