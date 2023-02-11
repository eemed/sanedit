use core::fmt;
use std::{collections::HashMap, sync::Arc};

use crate::server::{self, ClientId, FromJobs, JobFutureFn, JobId, JobProgress, JobsHandle};

use super::Editor;

pub(crate) type JobProgressFn = Arc<dyn Fn(&mut Editor, ClientId, Vec<String>) + Send + Sync>;

/// Holds the job itself that will be sent, and client side job data too.
pub(crate) struct AsyncJob {
    job: server::Job,
    progress_handlers: ProgressHandlers,
}

impl AsyncJob {
    pub fn new(
        id: ClientId,
        fun: JobFutureFn,
        on_output: Option<JobProgressFn>,
        on_error: Option<JobProgressFn>,
    ) -> AsyncJob {
        let server_job = server::Job::new(fun);
        AsyncJob {
            job: server_job,
            progress_handlers: ProgressHandlers {
                client_id: id,
                on_error,
                on_output,
            },
        }
    }
}

/// Holds progress functions and the client id that initiated this job.
pub(crate) struct ProgressHandlers {
    pub client_id: ClientId,
    pub on_error: Option<JobProgressFn>,
    pub on_output: Option<JobProgressFn>,
}

impl fmt::Debug for ProgressHandlers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("progress_handlers")
            .field("client_id", &self.client_id)
            .finish_non_exhaustive()
    }
}

#[derive(Debug)]
pub(crate) struct Jobs {
    handle: JobsHandle,
    running: HashMap<JobId, ProgressHandlers>,
}

impl Jobs {
    pub fn new(handle: JobsHandle) -> Jobs {
        Jobs {
            handle,
            running: HashMap::new(),
        }
    }

    pub fn run_job(&mut self, job: AsyncJob) {
        let AsyncJob {
            job,
            progress_handlers,
        } = job;
        let id = job.id();
        // Send the job to be ran
        self.handle.run_job(job);
        // Register progress handlers
        self.running.insert(id, progress_handlers);
    }

    pub fn on_output_handler(&self, id: &JobId) -> Option<(ClientId, JobProgressFn)> {
        let prog = self.running.get(id)?;
        let on_output = prog.on_output.clone()?;
        Some((prog.client_id, on_output))
    }

    pub fn on_error_handler(&self, id: &JobId) -> Option<(ClientId, JobProgressFn)> {
        let prog = self.running.get(id)?;
        let on_error = prog.on_error.clone()?;
        Some((prog.client_id, on_error))
    }

    pub fn job_done(&mut self, id: &JobId) {
        self.running.remove(id);
    }
}
