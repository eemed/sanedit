use core::fmt::Debug;
use std::{
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
    Progress(JobId, JobProgress),
    Ok(JobId),
    Fail(JobId),
}

#[derive(Debug)]
pub(crate) struct JobsHandle {
    send: mpsc::Sender<ToJobs>,
}

impl JobsHandle {
    pub fn run(&mut self, job: Job) -> Result<(), mpsc::error::SendError<ToJobs>> {
        self.send.blocking_send(ToJobs::New(job))
    }

    pub fn stop(&mut self, id: &JobId) -> Result<(), mpsc::error::SendError<ToJobs>> {
        self.send.blocking_send(ToJobs::Stop(*id))
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
    let task_handles: Arc<Mutex<HashMap<JobId, JoinHandle<()>>>> =
        Arc::new(Mutex::new(HashMap::new()));

    while let Some(msg) = recv.recv().await {
        match msg {
            ToJobs::New(job) => {
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
                        FromJobs::Ok(id)
                    } else {
                        FromJobs::Fail(id)
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
                    log::info!("Task stopped");
                    join.abort();
                }
            }
        }
    }
}
