#[derive(Debug)]
pub(crate) struct Options {
    /// When filesize is over this threshold it is considered big
    pub big_file_threshold_bytes: u64,
    pub project: ProjectOptions,
    pub ignore_directories: Vec<String>,
    pub shell: String,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            // big_file_threshold_bytes: 100 * 1024 * 1024, // 100MB
            big_file_threshold_bytes: 1024 * 1024, // 1MB
            project: ProjectOptions::default(),
            ignore_directories: vec![],
            shell: "/bin/bash".into(),
        }
    }
}

impl Options {
    pub fn ignore_directories(&self) -> Vec<String> {
        let mut ignore = self.ignore_directories.clone();
        ignore.extend_from_slice(&self.project.ignore_directories);
        ignore
    }
}

#[derive(Debug)]
pub(crate) struct ProjectOptions {
    pub build_command: String,
    pub run_command: String,
    pub ignore_directories: Vec<String>,
}

impl Default for ProjectOptions {
    fn default() -> Self {
        ProjectOptions {
            build_command: "task build".into(),
            run_command: "task run".into(),
            ignore_directories: vec![".git", "target"]
                .into_iter()
                .map(String::from)
                .collect(),
        }
    }
}
