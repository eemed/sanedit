use tokio::sync::mpsc::Sender;

use crate::{events::ToEditor, server::EditorHandle};

use super::{events::FromJobs, JobId};

pub(crate) enum InternalJobsMessage {
    Succesful(JobId),
    Failed(JobId),
}

/// Job context that contains handles required to communicate the jobs progress.
#[derive(Clone)]
pub(crate) struct JobContext {
    pub(crate) editor: EditorHandle,
    pub(crate) internal: Sender<InternalJobsMessage>,
}

impl JobContext {
    pub async fn success(
        &mut self,
        id: JobId,
    ) -> Result<(), tokio::sync::mpsc::error::SendError<InternalJobsMessage>> {
        self.editor
            .send(ToEditor::Jobs(FromJobs::Succesful(id)))
            .await;
        self.internal
            .send(InternalJobsMessage::Succesful(id))
            .await?;
        Ok(())
    }

    pub async fn failure(
        &mut self,
        id: JobId,
    ) -> Result<(), tokio::sync::mpsc::error::SendError<InternalJobsMessage>> {
        self.editor.send(ToEditor::Jobs(FromJobs::Failed(id))).await;
        self.internal.send(InternalJobsMessage::Failed(id)).await?;
        Ok(())
    }
}
