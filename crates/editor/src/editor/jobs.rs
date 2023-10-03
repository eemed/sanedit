use std::{any::Any, collections::HashMap, fmt, rc::Rc};

use crate::server::{ClientId, Job, JobId, JobsHandle, ToJobs};

use super::Editor;

/// A job that can talk back to the editor.
pub(crate) trait Talkative: Job {
    /// Ran when the job sends the message back to the editor
    fn on_message(&self, editor: &mut Editor, msg: Box<dyn Any>);
}

impl fmt::Debug for dyn Talkative {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "progressable job")
    }
}

#[derive(Debug)]
pub(crate) struct Jobs {
    handle: JobsHandle,
    jobs: HashMap<JobId, Rc<dyn Talkative>>,
}

impl Jobs {
    pub fn new(handle: JobsHandle) -> Jobs {
        Jobs {
            handle,
            jobs: HashMap::new(),
        }
    }

    /// Request a job that runs in the background
    pub fn request_job<T: Job + 'static>(&mut self, job: T) -> JobId {
        let job = job.box_clone();
        let id = JobId::next();
        let _ = self.handle.blocking_send(ToJobs::Request(id, job));
        id
    }

    /// Request a talkative job to be ran.
    pub fn request<T: Talkative + 'static>(&mut self, task: T) -> JobId {
        let job = task.box_clone();
        let id = JobId::next();
        let talkative = Rc::new(task);
        self.jobs.insert(id, talkative);
        let _ = self.handle.blocking_send(ToJobs::Request(id, job));
        id
    }

    pub fn done(&mut self, id: JobId) {
        self.jobs.remove(&id);
    }

    pub fn get(&self, id: JobId) -> Option<Rc<dyn Talkative>> {
        self.jobs.get(&id).cloned()
    }
}
