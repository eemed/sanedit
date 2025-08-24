use std::path::PathBuf;

pub mod unix;

#[derive(Debug)]
pub struct ClientOptions {
    pub session: String,
    pub file: Option<PathBuf>,
    pub parent_client: Option<usize>,
}
