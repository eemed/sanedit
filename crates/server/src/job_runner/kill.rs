use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

/// Wrapper around a atomic bool to stop multiple things in multiple threads at
/// once
#[derive(Debug, Clone, Default)]
pub struct KillSwitch {
    atomic: Arc<AtomicBool>,
}

impl KillSwitch {
    pub fn new() -> KillSwitch {
        Self::default()
    }

    pub fn is_killed(&self) -> bool {
        self.atomic.load(Ordering::Acquire)
    }

    pub fn kill(&self) {
        self.atomic.store(true, Ordering::Release)
    }
}

impl From<KillSwitch> for Arc<AtomicBool> {
    fn from(value: KillSwitch) -> Self {
        value.atomic
    }
}
