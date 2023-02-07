use crate::server::{Job, JobsHandle};

#[derive(Debug)]
pub(crate) struct Jobs {
    handle: JobsHandle,
}

impl Jobs {
    pub fn new(handle: JobsHandle) -> Jobs {
        Jobs { handle }
    }

    pub fn new_job(&mut self, job: Job) {
        self.handle.new_job(job);
    }
}
