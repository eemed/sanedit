use std::{
    cell::Cell,
    fs::{self, File, OpenOptions},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc, OnceLock,
    },
};

use rustc_hash::FxHashSet;

const TMP_DIR: &str = "tmp";
pub const SANE_DIR: &str = "sanedit";
pub const FILETYPE_DIR: &str = "filetype";
pub const THEME_DIR: &str = "themes";
pub const CONFIG: &str = "config.toml";

#[cfg(unix)]
pub const GLOBAL: &str = "/usr/share/sanedit";

#[cfg(not(unix))]
pub const GLOBAL: &str = "todo";

/// Directory that represents all the locations that contain the same
/// information. Used to find things from multiple places at once.
#[derive(Debug)]
pub struct Directory {
    dirs: Vec<PathBuf>,
}

impl Directory {
    pub fn find<A: AsRef<Path>>(&self, components: &[A]) -> Option<PathBuf> {
        for dir in &self.dirs {
            let mut d = dir.clone();
            d.extend(components);
            if d.exists() {
                return Some(d);
            }
        }

        None
    }

    pub fn find_all_files(&self) -> Vec<PathBuf> {
        let mut results = vec![];

        for dir in &self.dirs {
            if let Ok(mut rd) = std::fs::read_dir(dir) {
                while let Some(Ok(entry)) = rd.next() {
                    let Ok(ft) = entry.file_type() else {
                        continue;
                    };
                    if ft.is_file() {
                        results.push(entry.path());
                    }
                }
            }
        }

        results
    }

    pub fn find_all_distinct_files(&self) -> Vec<PathBuf> {
        let mut seen = FxHashSet::default();
        let mut results = vec![];

        for dir in &self.dirs {
            if let Ok(mut rd) = std::fs::read_dir(dir) {
                while let Some(Ok(entry)) = rd.next() {
                    let Ok(ft) = entry.file_type() else {
                        continue;
                    };
                    let path = entry.path();
                    let Ok(local) = path.strip_prefix(&dir) else {
                        continue;
                    };
                    if ft.is_file() && !seen.contains(local) {
                        seen.insert(local.to_path_buf());
                        results.push(path);
                    }
                }
            }
        }

        results
    }
}

#[derive(Debug)]
pub struct ConfigDirectory {
    dir: PathBuf,
}

impl ConfigDirectory {
    pub fn new(cd: &Path) -> ConfigDirectory {
        ConfigDirectory { dir: cd.into() }
    }

    pub fn filetype_dir(&self) -> Directory {
        let global = PathBuf::from(GLOBAL).join(FILETYPE_DIR);
        let local = self.dir.join(FILETYPE_DIR);
        Directory {
            dirs: vec![local, global],
        }
    }

    pub fn theme_dir(&self) -> Directory {
        let global = PathBuf::from(GLOBAL).join(THEME_DIR);
        let local = self.dir.join(THEME_DIR);
        Directory {
            dirs: vec![local, global],
        }
    }

    // TODO should be split into global and local config
    pub fn config(&self) -> PathBuf {
        let mut base = self.dir.clone();
        base.push(CONFIG);
        base
    }
}

impl Default for ConfigDirectory {
    fn default() -> Self {
        let cd = config_dir().expect("Failed to get configuration directory.");
        ConfigDirectory { dir: cd }
    }
}

// TODO remove these
pub fn config_dir() -> Option<PathBuf> {
    let config = dirs::config_dir()?;
    Some(config.join(SANE_DIR))
}

pub fn filetype_dir() -> Option<PathBuf> {
    let mut conf_dir = config_dir()?;
    conf_dir.push(FILETYPE_DIR);
    Some(conf_dir)
}

pub fn data_dir() -> Option<PathBuf> {
    let data = dirs::data_dir()?;
    Some(data.join(SANE_DIR))
}

pub fn tmp_dir() -> Option<PathBuf> {
    let data = data_dir()?;
    let tmp = data.join(TMP_DIR);

    if !tmp.exists() {
        fs::create_dir_all(&tmp).ok()?;
    }

    Some(tmp)
}

pub fn tmp_file() -> Option<(PathBuf, File)> {
    loop {
        let name = format!("tmp-file-{}", next_id());
        let mut path = tmp_dir()?;

        path.push(PathBuf::from(name));

        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(f) => return Some((path, f)),
            Err(e) => {
                use std::io::ErrorKind::*;

                match e.kind() {
                    PermissionDenied => return None,
                    _ => {}
                }
            }
        }
    }
}

fn next_id() -> u32 {
    static TMP_FILE_NUM: OnceLock<Arc<AtomicU32>> = OnceLock::new();
    let count = TMP_FILE_NUM.get_or_init(|| Arc::new(AtomicU32::new(0)));
    count.fetch_add(1, Ordering::Relaxed)
}
