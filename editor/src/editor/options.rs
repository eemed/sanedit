#[derive(Debug)]
pub(crate) struct Options {
    /// When filesize is over this threshold it is considered big
    pub big_file_threshold_bytes: u64,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            big_file_threshold_bytes: 100 * 1024 * 1024, // 100MB
        }
    }
}
