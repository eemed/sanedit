use std::collections::HashSet;

use super::inst::InstPtr;

pub type Thread = InstPtr;

pub struct ThreadSet {
    threads: HashSet<Thread>,
}

impl ThreadSet {
    pub fn add_thread(thread: Thread) {
        self.threads.insert(thread);
    }
}
