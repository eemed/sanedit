use std::path::PathBuf;

pub mod unix;

#[derive(Debug)]
pub struct ClientOptions {
    pub session: String,
    pub file: Option<InitialFile>,
    pub parent_client: Option<usize>,
}

#[derive(Debug)]
pub enum InitialFile {
    Path(PathBuf),
    Stdin(Vec<u8>)
}
