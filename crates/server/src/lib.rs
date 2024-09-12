mod events;
mod job_runner;
mod server;

pub use events::{FromEditor, ToEditor};
pub use job_runner::{
    spawn_job_runner, CPUJob, FromJobs, Job, JobContext, JobId, JobResult, JobsHandle, JobsMessage,
    Kill, ToJobs,
};
pub use server::{
    client::{ClientHandle, ClientId},
    spawn_listeners, Address, EditorHandle, StartOptions,
};

pub(crate) const CHANNEL_SIZE: usize = 256;
