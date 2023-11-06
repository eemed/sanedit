use std::{any::Any, collections::HashMap, fmt, rc::Rc};

use crate::server::{ClientId, Job, JobId, JobsHandle, ToJobs};

use super::Editor;

/// A job that keeps in touch (can send messages back) with the editor
pub(crate) trait KeepInTouch: Job {
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

    /// Request a job that runs in the background
    pub fn request_job<T: Job + 'static>(&mut self, job: T) -> JobId {
        let job = job.box_clone();
        let id = JobId::next();
        let _ = self.handle.blocking_send(ToJobs::Request(id, job));
        id
    }

    /// Request a job to be ran, tahat
    pub fn request<T: KeepInTouch + 'static>(&mut self, task: T) -> JobId {
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

    pub fn get(&self, id: JobId) -> Option<Rc<dyn KeepInTouch>> {
        self.jobs.get(&id).cloned()
    }
}
