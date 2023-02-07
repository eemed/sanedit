use core::fmt::Debug;
use std::{
    pin::Pin,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use futures::Future;
use tokio::sync::mpsc;

use crate::events::ToEditor;

use super::{EditorHandle, CHANNEL_SIZE};

#[derive(Debug)]
pub(crate) struct JobId {
    id: usize,
}

impl JobId {
    pub fn next() -> JobId {
        static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        JobId { id }
    }
}

// TODO give out a output provider instead of whole editorhandle, so that we can
// only send output and error messages from future fn
pub(crate) type JobFutureFn =
    Box<dyn Fn(EditorHandle) -> Pin<Box<dyn Future<Output = bool> + Send + Sync>> + Send + Sync>;

pub(crate) struct Job {
    id: JobId,
    fun: JobFutureFn,
}

impl Job {
    pub fn new(fun: JobFutureFn) -> Job {
        Job {
            id: JobId::next(),
            fun,
        }
    }
}

impl Debug for Job {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

#[derive(Debug)]
pub(crate) enum ToJobs {
    New(Job),
    Stop(JobId),
}

#[derive(Debug)]
pub(crate) enum FromJobs {
    Output(String),
    Error(String),
    Ok,
    Fail,
}

#[derive(Debug)]
pub(crate) struct JobsHandle {
    send: mpsc::Sender<ToJobs>,
}

impl JobsHandle {
    pub fn new_job(&mut self, job: Job) -> Result<(), mpsc::error::SendError<ToJobs>> {
        self.send.blocking_send(ToJobs::New(job))
    }
}

pub(crate) async fn spawn_jobs(editor_handle: EditorHandle) -> JobsHandle {
    let (tx, rx) = mpsc::channel(CHANNEL_SIZE);
    let handle = JobsHandle { send: tx };

    tokio::spawn(async {
        jobs_loop(rx, editor_handle).await;
    });

    handle
}

// Runs jobs in tokio runtime.
async fn jobs_loop(mut recv: mpsc::Receiver<ToJobs>, handle: EditorHandle) {
    while let Some(msg) = recv.recv().await {
        match msg {
            ToJobs::New(job) => {
                let job_future = (job.fun)(handle.clone());
                let mut h = handle.clone();
                let future = async move {
                    let success = job_future.await;
                    let msg = if success {
                        FromJobs::Ok
                    } else {
                        FromJobs::Fail
                    };
                    h.send(ToEditor::Jobs(msg)).await;
                };
                tokio::spawn(future);
            }
            ToJobs::Stop(id) => {
                // TODO implement stop mechanism for jobs
            }
        }
    }
}
