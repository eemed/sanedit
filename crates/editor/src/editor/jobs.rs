use core::fmt;
use std::{any::Any, collections::HashMap, sync::Arc};

use crate::server::{self, ClientId, JobFutureFn, JobId, JobsHandle};

use super::Editor;

pub(crate) type JobProgressFn = Arc<dyn Fn(&mut Editor, ClientId, Box<dyn Any>) + Send>;

/// Holds the job itself that will be sent, and client side job data too.
pub(crate) struct Job {
    job: server::JobRequest,
    handlers: ProgressHandlers,
}

impl Job {
    pub fn new(id: ClientId, fun: JobFutureFn) -> Job {
        let server_job = server::JobRequest::new(fun);
        Job {
            job: server_job,
            handlers: ProgressHandlers {
                client_id: id,
                on_error: None,
                on_output: None,
            },
        }
    }

    pub fn on_output(mut self, fun: JobProgressFn) -> Self {
        self.handlers.on_output = Some(fun);
        self
    }

    pub fn on_error(mut self, fun: JobProgressFn) -> Self {
        self.handlers.on_error = Some(fun);
        self
    }

    pub fn id(&self) -> JobId {
        self.job.id()
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

    pub fn request(&mut self, job: Job) -> JobId {
        let Job {
            job,
            handlers: progress_handlers,
        } = job;
        let id = job.id();
        // Send the job to be ran
        self.handle.request(job);
        // Register progress handlers
        self.running.insert(id, progress_handlers);
        id
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

    pub fn stop(&mut self, id: &JobId) {
        if let Some(_job) = self.running.remove(id) {
            let _ = self.handle.stop(id);
        }
    }

    pub fn done(&mut self, id: &JobId) {
        self.running.remove(id);
    }
}
