use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use sanedit_syntax::Glob;

use super::config::{Config, ProjectConfig};

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
    globs: Arc<Globs>,
}

impl Ignore {
    pub fn empty(working_dir: &Path) -> Ignore {
        let globs = Globs {
            root: working_dir.to_path_buf(),
            patterns: vec![],
        };
        Ignore { globs: Arc::new(globs) }
    }

    pub fn is_empty(&self) -> bool {
        self.globs.patterns.is_empty()
    }

    pub fn new(working_dir: &Path, config: &Config, project_config: &ProjectConfig) -> Ignore {
        let (patterns, root) = {
            let mut dir = project_config
                .project_file_path
                .as_ref()
                .map(|path| path.parent())
                .flatten()
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

        Ignore {
            globs: Arc::new(Globs {
                root,
                patterns: globs,
            }),
        }
    }

    pub fn is_match(&self, path: &Path) -> bool {
        let local = path.strip_prefix(&self.globs.root).unwrap_or(path);
        let bytes = local.to_string_lossy();
        self.is_match_bytes(bytes.as_bytes())
    }

    fn is_match_bytes(&self, bytes: &[u8]) -> bool {
        self.globs.patterns.iter().any(|glob| glob.is_match(&bytes))
    }
}
