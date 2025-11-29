use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use sanedit_syntax::Glob;

use super::config::{Config, ProjectConfig};

#[derive(Debug)]
pub(crate) struct Globs {
    pub(crate) root: PathBuf,
    pub(crate) patterns: Vec<Glob>,
}

/// Matches relative file paths.
/// Root is set to where the patterns are found
/// All matching strips the root prefix away
#[derive(Debug, Clone)]
pub(crate) struct Ignore {
    globs: Arc<Globs>,
    pub(crate) git_ignore: Arc<Vec<Globs>>,
}

impl Ignore {
    pub fn empty() -> Ignore {
        Ignore {
            globs: Arc::new(Globs {
                root: PathBuf::new(),
                patterns: vec![],
            }),
            git_ignore: Arc::new(vec![]),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.globs.patterns.is_empty()
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

    pub fn new(working_dir: &Path, config: &Config, project_config: &ProjectConfig) -> Ignore {
        let configured = Self::configured_ignores(working_dir, config, project_config);
        Ignore {
            globs: Arc::new(configured),
            git_ignore: Arc::new(vec![]),
        }
    }

    pub fn is_match(&self, path: &Path) -> bool {
        let local = path.strip_prefix(&self.globs.root).unwrap_or(path);
        let is_dir = path.is_dir();
        let bytes = local.to_string_lossy();

        for glob in &self.globs.patterns {
            let opts = glob.options();
            if opts.directory_only && !is_dir {
                continue;
            }

            if glob.is_match(&bytes.as_bytes()) {
                return !opts.negated;
            }
        }

        for ignore in self.git_ignore.as_ref() {
            for glob in &ignore.patterns {
                let opts = glob.options();
                if opts.directory_only && !is_dir {
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
