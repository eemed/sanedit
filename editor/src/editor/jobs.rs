mod fs;

pub(crate) use fs::*;

use std::collections::HashMap;

use crate::server::{ClientId, Job, JobId, JobsHandle};

#[derive(Debug)]
pub(crate) struct Jobs {
    handle: JobsHandle,
}

impl Jobs {
    pub fn new(handle: JobsHandle) -> Jobs {
        Jobs { handle }
    }

    pub fn new_job<J>(&mut self, job: J)
    where
        J: Job + 'static,
    {
        self.handle.new_job(job);
    }
}
