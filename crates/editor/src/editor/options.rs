#[derive(Debug)]
pub(crate) struct Options {
    /// When filesize is over this threshold it is considered big
    pub big_file_threshold_bytes: u64,
    pub project: ProjectOptions,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            // big_file_threshold_bytes: 100 * 1024 * 1024, // 100MB
            big_file_threshold_bytes: 1024 * 1024, // 1MB
            project: ProjectOptions::default(),
        }
    }
}

#[derive(Debug)]
pub(crate) struct ProjectOptions {
    pub build_command: String,
    pub run_command: String,
}

impl Default for ProjectOptions {
    fn default() -> Self {
        ProjectOptions {
            build_command: "task build".into(),
            run_command: "task run".into(),
        }
    }
}
