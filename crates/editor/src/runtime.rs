use tokio::{runtime::Runtime, task::JoinHandle};

use sanedit_server::{EditorHandle, Future};

#[derive(Debug)]
pub(crate) struct TokioRuntime {
    /// Tokio runtime
    tokio: Runtime,

    /// Handle to send messages to editor
    /// Spawned tasks need to communicate somehow
    handle: EditorHandle,
}

impl TokioRuntime {
    pub fn new(handle: EditorHandle) -> TokioRuntime {
        TokioRuntime {
            tokio: Runtime::new().unwrap(),
            handle,
        }
    }

    pub fn block_on<F: Future>(&self, fut: F) -> F::Output {
        self.tokio.block_on(fut)
    }

    #[allow(dead_code)]
    pub fn spawn<F>(&self, fut: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.tokio.spawn(fut)
    }

    pub fn editor_handle(&self) -> EditorHandle {
        self.handle.clone()
    }
}
