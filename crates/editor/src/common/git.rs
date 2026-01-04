use sanedit_syntax::GitGlob;

use std::{
    ffi::OsStr,
    fs::File,
    io::{BufRead as _, BufReader},
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
    sync::{Arc, OnceLock},
};

pub const GIT_DIR_NAME: &str = ".git";
pub const GIT_IGNORE_FILENAME: &str = ".gitignore";

pub fn git_dir() -> &'static OsStr {
    static S: OnceLock<&'static OsStr> = OnceLock::new();
    S.get_or_init(|| OsStr::new(GIT_DIR_NAME))
}

pub fn git_ignore() -> &'static OsStr {
    static S: OnceLock<&'static OsStr> = OnceLock::new();
    S.get_or_init(|| OsStr::new(GIT_IGNORE_FILENAME))
}

#[derive(Debug, Clone)]
pub(crate) struct GitIgnoreList(Vec<Arc<GitIgnore>>);

impl GitIgnoreList {
    pub fn new(ignore: Vec<Arc<GitIgnore>>) -> GitIgnoreList {
        GitIgnoreList(ignore)
    }

    pub fn is_ignored(&self, path: &Path) -> bool {
        for ignore in &self.0 {
            let Ok(local) = path.strip_prefix(&ignore.root) else {
                continue;
            };
            let bytes = local.to_string_lossy();

            for glob in &ignore.patterns {
                let opts = glob.options();
                if opts.directory_only && !path.is_dir() {
                    continue;
                }

                if glob.is_match(bytes.as_bytes()) {
                    return !opts.negated;
                }
            }
        }

        false
    }
}

impl Deref for GitIgnoreList {
    type Target = Vec<Arc<GitIgnore>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for GitIgnoreList {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug)]
pub(crate) struct GitIgnore {
    pub(crate) root: PathBuf,
    pub(crate) patterns: Vec<GitGlob>,
}

impl GitIgnore {
    pub fn new(path: &Path) -> std::io::Result<GitIgnore> {
        let mut patterns = vec![];
        let reader = BufReader::new(File::open(path)?);
        for line in reader.lines() {
            let line = line?;
            if line.starts_with("#") || line.is_empty() {
                continue;
            }

            match GitGlob::new(&line) {
                Ok(glob) => patterns.push(glob),
                Err(e) => log::error!("Failed to parse pattern: {} at {:?}: {}", line, &path, e),
            }
        }

        patterns.sort_by(|a, b| b.options().cmp(a.options()));

        Ok(GitIgnore {
            root: path
                .parent()
                .ok_or(std::io::Error::from(std::io::ErrorKind::NotFound))?
                .to_path_buf(),
            patterns,
        })
    }

    pub fn is_ignored(&self, path: &Path) -> bool {
        let Ok(local) = path.strip_prefix(&self.root) else {
            return false;
        };
        let bytes = local.to_string_lossy();

        for glob in &self.patterns {
            let opts = glob.options();
            if opts.directory_only && !path.is_dir() {
                continue;
            }

            if glob.is_match(bytes.as_bytes()) {
                return !opts.negated;
            }
        }

        false
    }
}

pub fn find_git_root(dir: &Path) -> std::io::Result<(PathBuf, Vec<GitIgnore>)> {
    let mut found_ignores = vec![];
    let mut dir = Some(dir);
    while let Some(d) = dir {
        if let Ok(mut rd) = d.read_dir() {
            let mut root = false;
            while let Some(Ok(entry)) = rd.next() {
                if entry.file_name().as_os_str() == git_ignore() {
                    if let Ok(ignore) = GitIgnore::new(&entry.path()) {
                        found_ignores.push(ignore);
                    }
                }

                if entry.file_name().as_os_str() == git_dir() {
                    root = true;
                }
            }

            if root {
                return Ok((d.to_path_buf(), found_ignores));
            }
        }

        dir = d.parent();
    }

    Err(std::io::ErrorKind::NotFound.into())
}
