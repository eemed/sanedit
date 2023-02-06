use core::fmt::{self, Debug};
use std::sync::Arc;

use tokio::sync::mpsc;

use crate::events::ToEditor;

use super::{EditorHandle, CHANNEL_SIZE};

#[derive(Debug)]
pub(crate) struct JobId {}

impl Default for JobId {
    fn default() -> Self {
        JobId {}
    }
}

pub(crate) struct Job {
    id: JobId,
    fun: Arc<dyn Fn(EditorHandle) + Send + Sync>,
}

impl Debug for Job {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Job").field("id", &self.id).finish()
    }
}

impl Default for Job {
    fn default() -> Self {
        Job {
            id: JobId::default(),
            fun: Arc::new(|mut handle| {
                tokio::spawn(async move {
                    let mut entries = std::fs::read_dir(".")
                        .unwrap()
                        .map(|res| res.map(|e| e.path()))
                        .collect::<Result<Vec<_>, std::io::Error>>()
                        .unwrap();
                    log::info!("Entries: {entries:?}");
                    handle.send(ToEditor::Jobs(FromJobs::Ok)).await;
                });
            }),
        }
    }
}

#[derive(Debug)]
pub(crate) enum ToJobs {
    New(Job),
    Stop(JobId),
}

#[derive(Debug)]
pub(crate) enum FromJobs {
    Ok,
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
async fn jobs_loop(mut recv: mpsc::Receiver<ToJobs>, mut handle: EditorHandle) {
    while let Some(msg) = recv.recv().await {
        match msg {
            ToJobs::New(Job { id, fun }) => {
                log::info!("new job");
                (fun)(handle.clone())
            }
            ToJobs::Stop(id) => {}
        }
    }
}
