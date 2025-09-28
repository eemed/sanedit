use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::read_toml;

const PROJECT_CONFIG: &str = "sanedit-project.toml";

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ProjectConfig {
    #[serde(skip)]
    pub(crate) project_file_path: Option<PathBuf>,
    pub(crate) run_command: String,
    pub(crate) build_command: String,
    pub(crate) ignore: Vec<String>,
}

impl ProjectConfig {
    pub fn new(working_dir: &Path) -> ProjectConfig {
        let path = {
            let mut config = working_dir.join(PROJECT_CONFIG);
            loop {
                if config.exists() {
                    break;
                } else {
                    // Try to go level up
                    // if no parent found return default conf
                    match config.parent().and_then(|parent| parent.parent()) {
                        Some(dir) => config = dir.join(PROJECT_CONFIG),
                        None => return Default::default(),
                    }
                }
            }
            config
        };

        match Self::try_new(&path) {
            Ok(mut config) => {
                config.project_file_path = path.into();
                config
            }
            Err(e) => {
                log::warn!("Failed to project configuration, using default instead: {e}");
                ProjectConfig::default()
            }
        }
    }

    pub fn try_new(config_path: &Path) -> anyhow::Result<ProjectConfig> {
        read_toml::<ProjectConfig>(config_path)
    }
}
