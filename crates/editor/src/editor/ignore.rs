use std::{
    ffi::OsStr,
    fs::File,
    io::{BufRead as _, BufReader},
    path::{Path, PathBuf},
    sync::{Arc, OnceLock},
};

use sanedit_syntax::Glob;

use super::config::{Config, ProjectConfig};

fn git_dir() -> &'static OsStr {
    static S: OnceLock<&'static OsStr> = OnceLock::new();
    S.get_or_init(|| OsStr::new(".git"))
}

fn git_ignore() -> &'static OsStr {
    static S: OnceLock<&'static OsStr> = OnceLock::new();
    S.get_or_init(|| OsStr::new(".gitignore"))
}

struct GitInfo {
    _root: PathBuf,
    ignores: Vec<Globs>,
}

impl GitInfo {
    pub fn new(working_dir: &Path) -> std::io::Result<GitInfo> {
        let git_root = Self::find_root(working_dir)?;
        let ignores = Self::find_ignores(&git_root)?;
        let ignores = Self::ignores_as_globs(ignores);

        Ok(GitInfo {
            _root: git_root,
            ignores,
        })
    }

    pub fn ignores_as_globs(ignores: Vec<PathBuf>) -> Vec<Globs> {
        let mut globs = Vec::with_capacity(ignores.len());

        for ignore in ignores {
            if let Ok(glob) = Self::read_ignore(ignore) {
                globs.push(glob);
            }
        }

        globs
    }

    fn read_ignore(path: PathBuf) -> std::io::Result<Globs> {
        let mut patterns = vec![];
        let reader = BufReader::new(File::open(&path)?);
        for line in reader.lines() {
            let line = line?;
            if line.starts_with("#") || line.is_empty() {
                continue;
            }

            match Glob::new(&line) {
                Ok(glob) => patterns.push(glob),
                Err(e) => log::error!("Failed to parse pattern: {} at {:?}: {}", line, &path, e),
            }
        }

        patterns.sort_by(|a, b| b.options().cmp(a.options()));

        Ok(Globs {
            root: path,
            patterns,
        })
    }

    fn find_ignores(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
        let mut results = vec![];
        let mut stack = vec![dir.to_path_buf()];
        while let Some(dir) = stack.pop() {
            if let Ok(mut rd) = std::fs::read_dir(&dir) {
                while let Some(Ok(entry)) = rd.next() {
                    let path = entry.path();

                    let Ok(metadata) = entry.metadata() else {
                        continue;
                    };

                    if metadata.is_dir() {
                        stack.push(path);
                    } else if path.file_name() == Some(git_ignore()) {
                        results.push(path);
                    }
                }
            }
        }

        Ok(results)
    }

    fn find_root(dir: &Path) -> std::io::Result<PathBuf> {
        let mut dir = Some(dir);
        while let Some(d) = dir {
            if let Ok(mut rd) = d.read_dir() {
                while let Some(Ok(entry)) = rd.next() {
                    if entry.file_name().as_os_str() == git_dir() {
                        return Ok(d.to_path_buf());
                    }
                }
            }

            dir = d.parent();
        }

        Err(std::io::ErrorKind::NotFound.into())
    }
}

#[derive(Debug)]
struct Globs {
    root: PathBuf,
    patterns: Vec<Glob>,
}

/// Matches relative file paths.
/// Root is set to where the patterns are found
/// All matching strips the root prefix away
#[derive(Debug, Clone)]
pub(crate) struct Ignore {
    globs: Arc<Vec<Globs>>,
}

impl Ignore {
    pub fn empty() -> Ignore {
        Ignore {
            globs: Arc::new(vec![]),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.globs.is_empty()
    }

    fn configured_ignores(
        working_dir: &Path,
        config: &Config,
        project_config: &ProjectConfig,
    ) -> Globs {
        let (patterns, root) = {
            let mut dir = project_config
                .project_file_path
                .as_ref()
                .and_then(|path| path.parent())
                .map(PathBuf::from)
                .unwrap_or_else(|| working_dir.into());
            let mut patts = &project_config.ignore;
            if patts.is_empty() {
                dir = working_dir.into();
                patts = &config.editor.ignore;
            }
            (patts, dir)
        };

        let mut globs = Vec::with_capacity(patterns.len());
        for patt in patterns {
            match Glob::new(patt.as_ref()) {
                Ok(glob) => globs.push(glob),
                Err(e) => log::error!("Failed to create ignore pattern: {e}"),
            }
        }

        Globs {
            root,
            patterns: globs,
        }
    }

    fn git_ignores(working_dir: &Path, config: &Config) -> Vec<Globs> {
        if !config.editor.git_ignore {
            return vec![];
        }

        if let Ok(git) = GitInfo::new(working_dir) {
            return git.ignores;
        }

        vec![]
    }

    pub fn new(working_dir: &Path, config: &Config, project_config: &ProjectConfig) -> Ignore {
        let mut globs = Self::git_ignores(working_dir, config);
        let configured = Self::configured_ignores(working_dir, config, project_config);
        globs.push(configured);

        Ignore {
            globs: Arc::new(globs),
        }
    }

    pub fn is_match(&self, path: &Path) -> bool {
        for globs in self.globs.as_ref() {
            let local = path.strip_prefix(&globs.root).unwrap_or(path);
            let bytes = local.to_string_lossy();

            for glob in &globs.patterns {
                let opts = glob.options();
                if opts.directory_only && !path.is_dir() {
                    continue;
                }

                if glob.is_match(&bytes.as_bytes()) {
                    return !opts.negated;
                }
            }
        }

        false
    }
}
