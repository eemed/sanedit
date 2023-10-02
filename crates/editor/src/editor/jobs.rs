use core::fmt;
use std::{any::Any, collections::HashMap, sync::Arc};

use crate::server::{self, ClientId, JobFutureFn, JobId, JobsHandle};

use super::Editor;

pub(crate) type JobProgressFn = Arc<dyn Fn(&mut Editor, ClientId, Box<dyn Any>) + Send>;

/// Holds the job itself that will be sent, and client side job data too.
pub(crate) struct Job {
    client_id: ClientId,
    job: server::JobRequest,
    pub on_error: Option<JobProgressFn>,
    pub on_output: Option<JobProgressFn>,
}

impl Job {
    pub fn new(id: ClientId, fun: JobFutureFn) -> Job {
        let server_job = server::JobRequest::new(fun);
        Job {
            client_id: id,
            job: server_job,
            on_error: None,
            on_output: None,
        }
    }

    pub fn id(&self) -> JobId {
        self.job.id()
    }
}

/// Holds progress functions and the client id that initiated this job.
pub(crate) struct Handlers {
    pub client_id: ClientId,
    pub on_error: Option<JobProgressFn>,
    pub on_output: Option<JobProgressFn>,
}

impl fmt::Debug for Handlers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Handlers").finish_non_exhaustive()
    }
}

#[derive(Debug)]
pub(crate) struct Jobs {
    handle: JobsHandle,
    running: HashMap<JobId, Handlers>,
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
            on_error,
            on_output,
            client_id,
        } = job;
        let id = job.id();
        // Send the job to be ran
        self.handle.request(job);

        let handlers = Handlers {
            client_id,
            on_error,
            on_output,
        };
        self.running.insert(id, handlers);
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
