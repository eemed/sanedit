use std::{env, fs, io, path::Path};

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(default)]
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

#[derive(Debug, Default, Deserialize)]
pub(crate) struct ProjectOptions {
    #[serde(default)]
    pub build_command: String,

    #[serde(default)]
    pub run_command: String,

    #[serde(default)]
    pub ignore_directories: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Config {
    pub editor: Options,
}

pub(crate) const PROJECT_CONFIG: &str = "sanedit-project.toml";

pub(crate) fn read_config(config: &Path) -> anyhow::Result<Config> {
    let mut config = read_toml::<Config>(config)?;

    let mut project_path = env::current_dir()?;
    project_path.push(PROJECT_CONFIG);

    match read_toml::<ProjectOptions>(&project_path) {
        Ok(pc) => config.editor.project = pc,
        Err(e) => {
            log::error!("Failed to read project settings: {e}");
        }
    }

    Ok(config)
}

fn read_toml<T: serde::de::DeserializeOwned>(toml: &Path) -> anyhow::Result<T> {
    use io::Read;

    let mut file = fs::File::open(toml)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;

    let toml = toml::from_str::<T>(&content)?;
    Ok(toml)
}
