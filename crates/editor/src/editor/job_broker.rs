mod cpu;

use std::{any::Any, collections::HashMap, fmt, rc::Rc};

use crate::job_runner::{Job, JobId, JobsHandle, ToJobs};
use crate::server::ClientId;

use super::Editor;

pub(crate) use cpu::{CPUJob, WrappedCPUJob};

/// A job that keeps in touch (can send messages back) with the editor
pub(crate) trait KeepInTouch {
    /// Ran when the job sends the message back to the editor
    fn on_message(&self, editor: &mut Editor, msg: Box<dyn Any>) {}
    fn on_success(&self, editor: &mut Editor) {}
    fn on_failure(&self, editor: &mut Editor, reason: &str) {}
    fn client_id(&self) -> ClientId;
}

impl fmt::Debug for dyn KeepInTouch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "keep in touch job")
    }
}

#[derive(Debug)]
pub(crate) struct JobBroker {
    handle: JobsHandle,
    jobs: HashMap<JobId, Rc<dyn KeepInTouch>>,
}

impl JobBroker {
    pub fn new(handle: JobsHandle) -> JobBroker {
        JobBroker {
            handle,
            jobs: HashMap::new(),
        }
    }

    /// Run a job in the background
    pub fn run_job<T>(&mut self, job: T) -> JobId
    where
        T: Job + Send + Sync + Clone + 'static,
    {
        let job = Box::new(job.clone());
        let id = JobId::next();
        let _ = self.handle.blocking_send(ToJobs::Request(id, job));
        id
    }

    /// Request a job to be ran
    pub fn request<T>(&mut self, task: T) -> JobId
    where
        T: Job + Send + Sync + Clone + KeepInTouch + 'static,
    {
        let job = Box::new(task.clone());
        let id = JobId::next();
        let talkative = Rc::new(task);
        self.jobs.insert(id, talkative);
        let _ = self.handle.blocking_send(ToJobs::Request(id, job));
        id
    }

    pub fn done(&mut self, id: JobId) {
        self.jobs.remove(&id);
    }

    pub fn stop(&mut self, id: JobId) {
        let _ = self.handle.blocking_send(ToJobs::Stop(id));
        self.done(id);
    }

    pub fn get(&self, id: JobId) -> Option<Rc<dyn KeepInTouch>> {
        self.jobs.get(&id).cloned()
    }
}
