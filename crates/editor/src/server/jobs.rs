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

use futures::Future;
use parking_lot::Mutex;
use tokio::{sync::mpsc, task::JoinHandle};

use crate::events::ToEditor;

use super::{EditorHandle, CHANNEL_SIZE};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

impl Display for JobId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.id)
    }
}

#[derive(Clone)]
pub(crate) struct JobProgressSender {
    pub(crate) id: JobId,
    pub(crate) handle: EditorHandle,
}

impl JobProgressSender {
    pub async fn send(
        &mut self,
        progress: JobProgress,
    ) -> Result<(), mpsc::error::SendError<ToEditor>> {
        self.handle
            .sender
            .send(ToEditor::Jobs(FromJobs::Progress(self.id, progress)))
            .await
    }
}

/// A Pinned future that resolves into an boolean indicating wether it succeeded
/// or failed
pub(crate) type PinnedFuture = Pin<Box<dyn Future<Output = bool> + Send>>;

/// a function that can be ran once, which produces the async future to be ran.
pub(crate) type JobFutureFn = Box<dyn FnOnce(JobProgressSender) -> PinnedFuture + Send>;

pub(crate) struct JobRequest {
    id: JobId,
    fun: JobFutureFn,
}

impl fmt::Debug for JobRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("JobRequest")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}

impl JobRequest {
    pub fn new(fun: JobFutureFn) -> JobRequest {
        JobRequest {
            id: JobId::next(),
            fun,
        }
    }

    pub fn id(&self) -> JobId {
        self.id
    }
}

#[derive(Debug)]
pub(crate) enum ToJobs {
    /// Request a new job to ran
    Request(JobRequest),
    /// Request to stop a job
    Stop(JobId),
}

/// Used to report job progress. Basically stdout and stderr, but with any
/// struct
#[derive(Debug)]
pub(crate) enum JobProgress {
    Output(Box<dyn Any + Send>),
    Error(Box<dyn Any + Send>),
}

#[derive(Debug)]
pub(crate) enum FromJobs {
    /// Sent when a job has made progress.
    Progress(JobId, JobProgress),

    /// Sent when a job succeeds.
    Completed(JobId),

    /// Sent when a job fails. Errors should be reported using
    /// `FromJobs::Progress` using `JobProgress::Error` variant.
    Failed(JobId),
}

#[derive(Debug)]
pub(crate) struct JobsHandle {
    send: mpsc::Sender<ToJobs>,
}

impl JobsHandle {
    pub fn request(&mut self, job: JobRequest) -> Result<(), mpsc::error::SendError<ToJobs>> {
        self.send.blocking_send(ToJobs::Request(job))
    }

    pub fn stop(&mut self, id: &JobId) -> Result<(), mpsc::error::SendError<ToJobs>> {
        self.send.blocking_send(ToJobs::Stop(*id))
    }
}

pub(crate) async fn spawn_jobs(editor_handle: EditorHandle) -> JobsHandle {
    let (tx, rx) = mpsc::channel(CHANNEL_SIZE);
    let handle = JobsHandle { send: tx };
    tokio::spawn(jobs_loop(rx, editor_handle));
    handle
}

// Runs jobs in tokio runtime.
async fn jobs_loop(mut recv: mpsc::Receiver<ToJobs>, handle: EditorHandle) {
    log::info!("jobs loop");
    let task_handles: Arc<Mutex<HashMap<JobId, JoinHandle<()>>>> =
        Arc::new(Mutex::new(HashMap::new()));

    while let Some(msg) = recv.recv().await {
        match msg {
            ToJobs::Request(job) => {
                let id = job.id;
                let progress_handle = JobProgressSender {
                    id,
                    handle: handle.clone(),
                };
                let job_future = (job.fun)(progress_handle);
                let mut h = handle.clone();
                let fut_jobs = task_handles.clone();
                let future = async move {
                    let success = job_future.await;
                    let msg = if success {
                        FromJobs::Completed(id)
                    } else {
                        FromJobs::Failed(id)
                    };
                    h.send(ToEditor::Jobs(msg)).await;

                    let mut map = fut_jobs.lock();
                    map.remove(&id);
                };

                let join = tokio::spawn(future);
                let mut map = task_handles.lock();
                map.insert(id, join);
            }
            ToJobs::Stop(id) => {
                let mut map = task_handles.lock();
                if let Some(join) = map.remove(&id) {
                    join.abort();
                }
            }
        }
    }
}
