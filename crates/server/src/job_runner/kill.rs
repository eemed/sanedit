use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

/// Wrapper around a atomic bool to stop multiple things in multiple threads at
/// once
#[derive(Clone, Default)]
pub struct Kill {
    atomic: Arc<AtomicBool>,
}

impl Kill {
    pub fn should_stop(&self) -> bool {
        self.atomic.load(Ordering::Relaxed)
    }

    pub fn stop(&self) {
        self.atomic.store(true, Ordering::Release)
    }
}

impl From<Kill> for Arc<AtomicBool> {
    fn from(value: Kill) -> Self {
        value.atomic
    }
}
