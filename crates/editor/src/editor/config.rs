use std::path::Path;

use serde::Deserialize;

use super::{buffers, options::Options, windows};

#[derive(Debug, Deserialize)]
pub(crate) struct Config {
    pub editor: Options,
    pub window: windows::Options,
    pub file: buffers::Options,
}

pub(crate) const PROJECT_CONFIG: &str = "sanedit-project.toml";

pub(crate) fn read_config(config_path: &Path, working_dir: &Path) -> anyhow::Result<Config> {
    let mut local = working_dir.to_path_buf();
    local.push(PROJECT_CONFIG);

    let config = config::Config::builder()
        .add_source(config::File::from(config_path))
        .add_source(config::File::from(local))
        .build()?;

    let config = config.try_deserialize::<Config>()?;

    Ok(config)
}
