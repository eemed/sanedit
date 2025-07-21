use std::{
    fmt::Display,
    sync::atomic::{AtomicUsize, Ordering},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct JobId {
    id: usize,
}

impl JobId {
    pub fn next() -> JobId {
        static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        JobId { id }
    }

    pub fn as_usize(&self) -> usize {
        self.id
    }
}

impl From<usize> for JobId {
    fn from(value: usize) -> Self {
        JobId { id: value }
    }
}

impl Display for JobId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.id)
    }
}
