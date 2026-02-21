use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use sanedit_syntax::GitGlob;

use crate::common::git::GitIgnore;

use super::config::{Config, ProjectConfig};

/// Manually added git ignore patterns
/// Does not depend on being in a git repository
#[derive(Debug, Clone)]
pub(crate) struct Ignore {
    globs: Arc<GitIgnore>,
}

impl Ignore {
    pub fn empty() -> Ignore {
        Ignore {
            globs: Arc::new(GitIgnore {
                root: PathBuf::new(),
                patterns: vec![],
            }),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.globs.patterns.is_empty()
    }

    fn configured_ignores(
        working_dir: &Path,
        config: &Config,
        project_config: &ProjectConfig,
    ) -> GitIgnore {
        let (patterns, root) = {
            let mut dir = project_config
                .project_config_path
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
            match GitGlob::new(patt.as_ref()) {
                Ok(glob) => globs.push(glob),
                Err(e) => log::error!("Failed to create ignore pattern: {e}"),
            }
        }

        GitIgnore {
            root,
            patterns: globs,
        }
    }

    pub fn new(working_dir: &Path, config: &Config, project_config: &ProjectConfig) -> Ignore {
        let configured = Self::configured_ignores(working_dir, config, project_config);
        Ignore {
            globs: Arc::new(configured),
        }
    }

    pub fn is_ignored(&self, path: &Path) -> bool {
        self.globs.is_ignored(path)
    }
}

impl From<Ignore> for Arc<GitIgnore> {
    fn from(value: Ignore) -> Self {
        value.globs
    }
}
