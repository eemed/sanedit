use std::path::PathBuf;

pub mod unix;

#[derive(Debug)]
pub struct SocketStartOptions {
    pub file: Option<PathBuf>,
}
