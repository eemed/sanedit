
use crate::{
    events::ToEditor,
    server::{FromJobs, Job},
};

#[derive(Debug)]
pub struct ListFiles {}

impl Job for ListFiles {
    fn run_async(&mut self, mut handle: crate::server::EditorHandle) {
        tokio::spawn(async move {
            log::info!("running list files job");
        });
    }
}

impl Default for ListFiles {
    fn default() -> Self {
        ListFiles {}
    }
}
