use core::fmt::Debug;
use std::sync::atomic::{AtomicUsize, Ordering};

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

/// Job trait, tokio runtime is used to run these
pub(crate) trait Job: Debug + Send {
    fn run_async(&mut self, handle: EditorHandle);
}

#[derive(Debug)]
pub(crate) enum ToJobs {
    New(Box<dyn Job>),
    Stop(JobId),
}

#[derive(Debug)]
pub(crate) enum FromJobs {
    Output(String),
    Error(String),
    Done,
}

#[derive(Debug)]
pub(crate) struct JobsHandle {
    send: mpsc::Sender<ToJobs>,
}

impl JobsHandle {
    pub fn new_job<J>(&mut self, job: J) -> Result<(), mpsc::error::SendError<ToJobs>>
    where
        J: Job + 'static,
    {
        self.send.blocking_send(ToJobs::New(Box::new(job)))
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
async fn jobs_loop(mut recv: mpsc::Receiver<ToJobs>, mut handle: EditorHandle) {
    while let Some(msg) = recv.recv().await {
        match msg {
            ToJobs::New(mut job) => {
                let job = job.as_mut();
                job.run_async(handle.clone());
                handle.send(ToEditor::Jobs(FromJobs::Done)).await;
            }
            ToJobs::Stop(id) => {
                // TODO implement stop mechanism for jobs
            }
        }
    }
}
