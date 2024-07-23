use std::any::Any;

use crate::events::ToEditor;
use crate::server::EditorHandle;
use tokio::sync::{
    broadcast,
    mpsc::{error::SendError, Sender},
};

use super::{FromJobs, JobId};

/// Job context used to provide jobs the means to communicate back to the
/// editor.
pub(crate) struct JobContext {
    pub id: JobId,
    pub kill: broadcast::Sender<()>,
    pub sender: JobResponseSender,
}

impl JobContext {
    pub fn send<A: Any + Send>(&mut self, any: A) {
        let any = Box::new(any);
        self.sender
            .editor
            .send(ToEditor::Jobs(FromJobs::Message(self.id, any)));
    }
}

/// Used for internal messaging when the job is completed
pub(super) enum JobsMessage {
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
pub(crate) struct JobResponseSender {
    pub(super) editor: EditorHandle,
    pub(super) internal: Sender<JobsMessage>,
}

impl JobResponseSender {
    pub(super) fn to_job_context(&self, id: JobId) -> (broadcast::Sender<()>, JobContext) {
        let (tx, rx) = broadcast::channel(1);
        let sender = self.clone();
        (
            tx.clone(),
            JobContext {
                id,
                sender,
                kill: tx,
            },
        )
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
        self.editor.send(ToEditor::Jobs(FromJobs::Message(id, any)));
    }
}

impl From<JobContext> for JobResponseSender {
    fn from(ctx: JobContext) -> Self {
        ctx.sender
    }
}
