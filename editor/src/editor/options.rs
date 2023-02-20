#[derive(Debug)]
pub(crate) struct EditorOptions {
    pub big_file_threshold_bytes: u64,
}

impl Default for EditorOptions {
    fn default() -> Self {
        EditorOptions {
            big_file_threshold_bytes: 100 * 1024 * 1024, // 100MB
        }
    }
}
