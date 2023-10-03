use super::{BoxedJob, JobId};

pub(crate) enum ToJobs {
    /// Request a new job to ran
    Request(JobId, BoxedJob),
    /// Request to stop a job
    Stop(JobId),
}

#[derive(Debug)]
pub(crate) enum FromJobs {
    // /// Sent when a job has made progress.
    // Progress(JobId, JobProgress),
    /// Sent when a job succeeds.
    Succesful(JobId),

    /// Sent when a job fails. Errors should be reported using
    /// `FromJobs::Progress` using `JobProgress::Error` variant.
    Failed(JobId),
}
