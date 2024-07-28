use documented::DocumentedFields;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, DocumentedFields)]
#[serde(default)]
pub(crate) struct Options {
    ///
    /// Large file threshold in bytes
    ///
    pub big_file_threshold_bytes: u64,
    ///
    /// Directories to ignore, when opening files etc.
    ///
    pub ignore_directories: Vec<String>,
    ///
    /// Default shell command
    ///
    pub shell: String,
    ///
    /// Shell command to build current project
    ///
    pub build_command: String,
    ///
    /// Shell command to run current project
    ///
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
        let ignore = self.ignore_directories.clone();
        ignore
    }
}
