use crate::server::{Job, JobsHandle};

#[derive(Debug)]
pub(crate) struct Jobs {
    handle: JobsHandle,
}

impl Jobs {
    pub fn new(handle: JobsHandle) -> Jobs {
        Jobs { handle }
    }

    pub fn run_job(&mut self, job: Job) {
        self.handle.run_job(job);
    }
}
