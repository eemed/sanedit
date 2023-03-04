use std::collections::HashSet;

use super::inst::InstPtr;

pub type Thread = InstPtr;

pub struct ThreadSet {
    threads: HashSet<Thread>,
}

impl ThreadSet {
    pub fn with_capacity(cap: usize) -> ThreadSet {
        ThreadSet { threads: HashSet::with_capacity(cap) }
    }

    pub fn add_thread(&mut self, thread: Thread) {
        self.threads.insert(thread);
    }

    pub fn clear(&mut self) {
        self.threads.clear();
    }
}
