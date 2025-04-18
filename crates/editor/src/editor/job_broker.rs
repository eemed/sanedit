use std::{any::Any, fmt, rc::Rc};

use sanedit_server::{ClientId, Job, JobId, JobsHandle, ToJobs};

use super::Editor;

use rustc_hash::FxHashMap;

/// A job that can send messages back to the editor
pub(crate) trait KeepInTouch {
    /// Ran when the job sends the message back to the editor
    fn on_message(&self, _editor: &mut Editor, _msg: Box<dyn Any>) {}
    fn on_success(&self, _editor: &mut Editor) {}
    fn on_failure(&self, _editor: &mut Editor, _reason: &str) {}
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
    jobs: FxHashMap<JobId, Rc<dyn KeepInTouch>>,

    /// Slots to store one job id that should be cancelled before another one
    /// with the same client id + string combo is requested.
    slots: FxHashMap<(ClientId, String), JobId>,
}

impl JobBroker {
    pub fn new(handle: JobsHandle) -> JobBroker {
        JobBroker {
            handle,
            jobs: FxHashMap::default(),
            slots: FxHashMap::default(),
        }
    }

    #[allow(dead_code)]
    /// Run a job in the background
    pub fn run<T>(&mut self, job: T) -> JobId
    where
        T: Job + Send + Sync + Clone + 'static,
    {
        let job = Box::new(job.clone());
        let id = JobId::next();
        let _ = self.handle.blocking_send(ToJobs::Request(id, job));
        id
    }

    /// Request a job to be ran in a slot.
    /// If the slot (id, name) pair already contains a job stop it.
    pub fn request_slot<T>(&mut self, id: ClientId, name: &str, task: T) -> JobId
    where
        T: Job + Send + Sync + Clone + KeepInTouch + 'static,
    {
        self.stop_slot(id, name);

        let key = (id, name.to_string());
        let jid = self.request(task);
        self.slots.insert(key, jid);
        jid
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

    pub fn stop_slot(&mut self, id: ClientId, name: &str) {
        let key = (id, name.to_string());
        if let Some(jid) = self.slots.remove(&key) {
            self.stop(jid);
        }
    }

    pub fn stop(&mut self, id: JobId) {
        if self.jobs.remove(&id).is_some() {
            let _ = self.handle.blocking_send(ToJobs::Stop(id));
        }
    }

    pub fn get(&self, id: JobId) -> Option<Rc<dyn KeepInTouch>> {
        self.jobs.get(&id).cloned()
    }
}
