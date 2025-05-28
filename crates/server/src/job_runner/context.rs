use std::any::Any;

use crate::events::ToEditor;
use crate::server::EditorHandle;
use tokio::sync::mpsc::{error::SendError, Sender};

use super::{FromJobs, JobId, Kill};

/// Job context used to provide jobs the means to communicate back to the
/// editor.
pub struct JobContext {
    pub id: JobId,
    pub kill: Kill,
    pub sender: JobResponseSender,
}

impl JobContext {
    pub fn send<A: Any + Send>(&mut self, any: A) {
        let any = Box::new(any);
        self.sender
            .editor
            .send(ToEditor::Jobs(FromJobs::Message(self.id, any)))
            .expect("Main loop shut down")
    }
}

/// Used for internal messaging when the job is completed
pub enum JobsMessage {
    Succesful(JobId),
    Failed(JobId, String),
}

impl JobsMessage {
    pub fn id(&self) -> JobId {
        match self {
            JobsMessage::Succesful(id) => *id,
            JobsMessage::Failed(id, _) => *id,
        }
    }
}

impl From<JobsMessage> for FromJobs {
    fn from(value: JobsMessage) -> Self {
        match value {
            JobsMessage::Succesful(id) => FromJobs::Succesful(id),
            JobsMessage::Failed(id, reason) => FromJobs::Failed(id, reason),
        }
    }
}

/// Job context used to communicate back to editor
#[derive(Clone, Debug)]
pub struct JobResponseSender {
    pub(super) editor: crossbeam::channel::Sender<ToEditor>,
    pub(super) internal: Sender<JobsMessage>,
}

impl JobResponseSender {
    pub(super) fn to_job_context(&self, id: JobId) -> JobContext {
        let kill = Kill::default();
        let sender = self.clone();
        JobContext { id, sender, kill }
    }

    pub(super) async fn success(&mut self, id: JobId) -> Result<(), SendError<JobsMessage>> {
        self.internal.send(JobsMessage::Succesful(id)).await?;
        Ok(())
    }

    pub(super) async fn failure(
        &mut self,
        id: JobId,
        reason: String,
    ) -> Result<(), SendError<JobsMessage>> {
        self.internal.send(JobsMessage::Failed(id, reason)).await?;
        Ok(())
    }

    pub fn send<A: Any + Send>(&mut self, id: JobId, any: A) {
        let any = Box::new(any);
        let _ = self.editor.send(ToEditor::Jobs(FromJobs::Message(id, any)));
    }
}

impl From<JobContext> for JobResponseSender {
    fn from(ctx: JobContext) -> Self {
        ctx.sender
    }
}
