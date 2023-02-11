use core::fmt::Debug;
use std::{
    pin::Pin,
    sync::atomic::{AtomicUsize, Ordering},
};

use futures::Future;
use tokio::sync::mpsc;

use crate::events::ToEditor;

use super::{EditorHandle, CHANNEL_SIZE};

#[derive(Debug, Clone, Copy)]
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

#[derive(Clone)]
pub(crate) struct JobProgressSender(EditorHandle);

impl JobProgressSender {
    pub async fn send(
        &mut self,
        progress: JobProgress,
    ) -> Result<(), mpsc::error::SendError<ToEditor>> {
        self.0
            .sender
            .send(ToEditor::Jobs(FromJobs::Progress(progress)))
            .await
    }
}

pub(crate) type PinnedFuture = Pin<Box<dyn Future<Output = bool> + Send + Sync>>;
pub(crate) type JobFutureFn = Box<dyn FnOnce(JobProgressSender) -> PinnedFuture + Send + Sync>;

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

    pub fn id(&self) -> JobId {
        self.id
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

/// Used to report job progress. Basically stdout and stderr.
#[derive(Debug)]
pub(crate) enum JobProgress {
    Output(Vec<String>),
    Error(Vec<String>),
}

#[derive(Debug)]
pub(crate) enum FromJobs {
    Progress(JobProgress),
    Ok,
    Fail,
}

#[derive(Debug)]
pub(crate) struct JobsHandle {
    send: mpsc::Sender<ToJobs>,
}

impl JobsHandle {
    pub fn run_job(&mut self, job: Job) -> Result<(), mpsc::error::SendError<ToJobs>> {
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
                let progress_handle = JobProgressSender(handle.clone());
                let job_future = (job.fun)(progress_handle);
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
