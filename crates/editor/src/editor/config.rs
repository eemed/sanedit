use std::{env, fs, io, path::Path};

use serde::Deserialize;

use super::{options::Options, windows};

#[derive(Debug, Deserialize)]
pub(crate) struct Config {
    pub editor: Options,
    pub window: windows::Options,
}

pub(crate) const PROJECT_CONFIG: &str = "sanedit-project.toml";

pub(crate) fn read_config(config: &Path) -> anyhow::Result<Config> {
    let config = read_toml::<Config>(config)?;

    let mut project_path = env::current_dir()?;
    project_path.push(PROJECT_CONFIG);

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
