use tokio::sync::mpsc;

use crate::{
    events::ToEditor,
    server::{Job, JobId, JobsHandle, ToJobs},
};

#[derive(Debug)]
pub(crate) struct Jobs {
    handle: JobsHandle,
}

impl Jobs {
    pub fn new(handle: JobsHandle) -> Jobs {
        Jobs { handle }
    }

    pub fn test(&mut self) {
        self.handle.new_job(Job::default());
    }
}
