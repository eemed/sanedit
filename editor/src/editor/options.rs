/// Describes when to convert files from utf8 to other encodings.
#[derive(Debug)]
pub enum Convert {
    Always,
    Ask,
    Never,
}

#[derive(Debug)]
pub(crate) struct EditorOptions {
    /// When filesize is over this threshold it is considered big
    pub big_file_threshold_bytes: u64,
    pub convert_small: Convert,
    pub convert_big: Convert,
}

impl Default for EditorOptions {
    fn default() -> Self {
        EditorOptions {
            big_file_threshold_bytes: 100 * 1024 * 1024, // 100MB
            auto_small_to_utf8: true,
        }
    }
}
