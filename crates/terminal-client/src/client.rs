use std::path::PathBuf;

pub mod unix;

#[derive(Debug, Clone)]
pub struct ClientOptions {
    pub session: String,
    pub file: Option<InitialFile>,
    pub parent_client: Option<usize>,
    pub language: Option<String>,
}

#[derive(Debug, Clone)]
pub enum InitialFile {
    Path(PathBuf),
    Stdin(Vec<u8>),
}
