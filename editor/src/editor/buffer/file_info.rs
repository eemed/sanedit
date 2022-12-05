use std::path::PathBuf;

#[derive(Debug, Clone)]
pub(crate) struct FileInfo {
    pub(crate) filename: String,
    pub(crate) absolute_path: PathBuf,
    pub(crate) encoding: &'static encoding_rs::Encoding,
    pub(crate) size: u64,
}
