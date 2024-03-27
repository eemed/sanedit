use std::{
    fs,
    path::{Path, PathBuf},
};

pub(crate) const ENV_PREFIX: &str = "SANE";

const TMP_DIR: &str = "tmp";
pub(crate) const SANE_DIR: &str = "sane";
pub(crate) const FILETYPE_DIR: &str = "filetype";
pub(crate) const THEME_DIR: &str = "themes";

#[derive(Debug)]
pub(crate) struct ConfigDirectory {
    dir: PathBuf,
}

impl ConfigDirectory {
    pub fn new(cd: &Path) -> ConfigDirectory {
        ConfigDirectory { dir: cd.into() }
    }

    pub fn filetype_dir(&self) -> PathBuf {
        let mut base = self.dir.clone();
        base.push(FILETYPE_DIR);
        base
    }

    pub fn theme_dir(&self) -> PathBuf {
        let mut base = self.dir.clone();
        base.push(THEME_DIR);
        base
    }
}

impl Default for ConfigDirectory {
    fn default() -> Self {
        let cd = config_dir().expect("Failed to get configuration directory.");
        ConfigDirectory { dir: cd }
    }
}

pub(crate) fn config_dir() -> Option<PathBuf> {
    let config = dirs::config_dir()?;
    Some(config.join(SANE_DIR))
}

pub(crate) fn filetype_dir() -> Option<PathBuf> {
    let mut conf_dir = config_dir()?;
    conf_dir.push(FILETYPE_DIR);
    Some(conf_dir)
}

pub(crate) fn data_dir() -> Option<PathBuf> {
    let data = dirs::data_dir()?;
    Some(data.join(SANE_DIR))
}

pub(crate) fn tmp_dir() -> Option<PathBuf> {
    let data = data_dir()?;
    let tmp = data.join(TMP_DIR);

    if !tmp.exists() {
        fs::create_dir_all(&tmp).ok()?;
    }

    Some(tmp)
}

pub(crate) fn tmp_file() -> Option<PathBuf> {
    let id = uuid::Uuid::new_v4();
    let mut result = tmp_dir()?;
    result.push(PathBuf::from(id.to_string()));
    Some(result)
}
