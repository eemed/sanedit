use std::{fs, path::PathBuf};

use crate::server::ClientId;

pub(crate) const ENV_PREFIX: &str = "SANE";

const TMP_DIR: &str = "tmp";
const SANE_DIR: &str = "sane";

pub(crate) fn config_dir() -> Option<PathBuf> {
    let config = dirs::config_dir()?;
    Some(config.join(SANE_DIR))
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
    // TODO another way without uuids?
    let id = uuid::Uuid::new_v4();
    let mut result = tmp_dir()?;
    result.push(PathBuf::from(id.to_string()));
    log::info!("tmp file: {result:?}");
    Some(result)
}

pub(crate) fn tmp_file2(id: ClientId, name: &str) -> Option<PathBuf> {
    let mut result = tmp_dir()?;
    let mut fname = {
        let n = String::from(id);
        n.push_str("-");
        n.push_str(name);
        n
    };
    let fpath = PathBuf::from(fname);
    result.push(fpath);
    Some(result)
}
