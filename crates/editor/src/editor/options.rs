use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(default)]
pub(crate) struct Options {
    /// When filesize is over this threshold it is considered big
    pub big_file_threshold_bytes: u64,
    pub ignore_directories: Vec<String>,
    pub shell: String,
    pub build_command: String,
    pub run_command: String,
    pub detect_eol: bool,
    pub detect_indent: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            // big_file_threshold_bytes: 100 * 1024 * 1024, // 100MB
            big_file_threshold_bytes: 1024 * 1024, // 1MB
            ignore_directories: vec![],
            shell: "/bin/bash".into(),
            build_command: String::new(),
            run_command: String::new(),
            detect_eol: true,
            detect_indent: true,
        }
    }
}

impl Options {
    pub fn ignore_directories(&self) -> Vec<String> {
        let mut ignore = self.ignore_directories.clone();
        ignore
    }
}
