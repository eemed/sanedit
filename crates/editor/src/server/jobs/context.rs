use std::any::Any;

use super::{events::FromJobs, JobId};
use crate::{events::ToEditor, server::EditorHandle};
use tokio::sync::mpsc::Sender;

pub(crate) use internal::*;

/// Job context used to provide jobs the means to communicate back to the
/// editor.
#[derive(Clone)]
pub(crate) struct JobContext {
    inner: InternalJobContext,
}

impl JobContext {
    pub fn send<A: Any + Send>(&mut self, id: JobId, any: A) {
        let any = Box::new(any);
        self.inner
            .editor
            .send(ToEditor::Jobs(FromJobs::Message(id, any)));
    }
}

impl From<InternalJobContext> for JobContext {
    fn from(inner: InternalJobContext) -> Self {
        JobContext { inner }
    }
}

impl From<JobContext> for InternalJobContext {
    fn from(ctx: JobContext) -> Self {
        ctx.inner
    }
}

mod internal {
    use super::JobId;
    use crate::{
        events::ToEditor,
        server::{EditorHandle, FromJobs},
    };
    use tokio::sync::mpsc::{error::SendError, Sender};

    /// Used for internal messaging when the job is completed
    pub(crate) enum InternalJobsMessage {
        Succesful(JobId),
        Failed(JobId, String),
    }

    impl InternalJobsMessage {
        pub fn id(&self) -> JobId {
            match self {
                InternalJobsMessage::Succesful(id) => *id,
                InternalJobsMessage::Failed(id, _) => *id,
            }
        }
    }

    impl From<InternalJobsMessage> for FromJobs {
        fn from(value: InternalJobsMessage) -> Self {
            match value {
                InternalJobsMessage::Succesful(id) => FromJobs::Succesful(id),
                InternalJobsMessage::Failed(id, reason) => FromJobs::Failed(id, reason),
            }
        }
    }

    /// Job context used to communicate internally
    #[derive(Clone)]
    pub(crate) struct InternalJobContext {
        pub(crate) editor: EditorHandle,
        pub(crate) internal: Sender<InternalJobsMessage>,
    }

    impl InternalJobContext {
        pub async fn success(&mut self, id: JobId) -> Result<(), SendError<InternalJobsMessage>> {
            self.internal
                .send(InternalJobsMessage::Succesful(id))
                .await?;
            Ok(())
        }

        pub async fn failure(
            &mut self,
            id: JobId,
            reason: String,
        ) -> Result<(), SendError<InternalJobsMessage>> {
            self.internal
                .send(InternalJobsMessage::Failed(id, reason))
                .await?;
            Ok(())
        }
    }
}
