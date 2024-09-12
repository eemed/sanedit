use std::any::Any;

use super::{BoxedJob, JobId};

pub enum ToJobs {
    /// Request a new job to ran
    Request(JobId, BoxedJob),
    /// Request to stop a job
    Stop(JobId),
}

#[derive(Debug)]
pub enum FromJobs {
    /// Message from a job. Could be anything.
    Message(JobId, Box<dyn Any + Send>),

    /// Sent when a job succeeds.
    Succesful(JobId),

    /// Sent when a job fails with a reason why it failed.
    Failed(JobId, String),
}
