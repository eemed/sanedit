use std::{
    io::Read as _,
    path::{Path, PathBuf},
    sync::{Arc, OnceLock},
};

use rustc_hash::FxHashMap;
use sanedit_syntax::Glob;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Detect {
    extensions: Vec<String>,
    glob: Vec<String>,
    shebang: Vec<String>,

    #[serde(skip)]
    compiled_globs: Arc<OnceLock<Vec<Glob>>>,
}

impl Detect {
    pub fn new(extensions: Vec<String>, globs: Vec<String>, shebangs: Vec<String>) -> Detect {
        Detect {
            extensions,
            glob: globs,
            shebang: shebangs,
            compiled_globs: Arc::new(OnceLock::new()),
        }
    }

    pub fn compile_globs(&self) -> &Vec<Glob> {
        self.compiled_globs.get_or_init(move || {
            let mut globs = Vec::with_capacity(self.glob.len());
            for pat in &self.glob {
                if let Ok(glob) = Glob::new(pat) {
                    globs.push(glob);
                }
            }
            globs
        })
    }

    pub fn merge(&mut self, detect: Detect) {
        let extensions = std::mem::take(&mut self.extensions);
        self.extensions.extend(detect.extensions);
        self.extensions.dedup();

        let glob = std::mem::take(&mut self.glob);
        self.glob.extend(detect.glob);
        self.glob.dedup();

        let shebang = std::mem::take(&mut self.shebang);
        self.shebang.extend(detect.shebang);
        self.shebang.dedup();

        *self = Detect::new(extensions, glob, shebang);
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Ord, PartialOrd, Clone)]
pub struct Language {
    name: String,
}

impl Language {
    /// Determine a filetype from a pattern,
    /// All of the below should be detected correctly:
    ///
    /// * Language name itself eg. "rust"
    /// * A file path "/path/to/language.rs"
    /// * Extension of a file "rs"
    pub fn determine_str(pattern: &str, patterns: &FxHashMap<String, Detect>) -> Option<Language> {
        let path = PathBuf::from(pattern);
        let mut buf = [0u8; 128];

        for (ft, detect) in patterns {
            if ft == pattern {
                return Some(Language {
                    name: ft.to_string(),
                });
            }

            if let Some(ext) = path.extension() {
                if let Some(ext) = ext.to_str() {
                    for dext in &detect.extensions {
                        if ext == dext {
                            return Some(Language {
                                name: ft.to_string(),
                            });
                        }
                    }
                }
            }

            for glob in detect.compile_globs() {
                if glob.is_match(&pattern.as_bytes()) {
                    return Some(Language {
                        name: ft.to_string(),
                    });
                }
            }

            let n = std::fs::File::open(&path)
                .ok()
                .map(|mut file| file.read(&mut buf).ok())
                .flatten();
            if let Some(n) = n {
                let read = &buf[..n];
                for shebang in &detect.shebang {
                    if read.starts_with(shebang.as_bytes()) {
                        return Some(Language {
                            name: ft.to_string(),
                        });
                    }
                }
            }
        }

        if let Some(ext) = path.extension() {
            let lang = ext.to_string_lossy();
            return Some(Language { name: lang.into() });
        }

        Some(Language { name: pattern.into() })
    }

    pub fn determine(path: &Path, patterns: &FxHashMap<String, Detect>) -> Option<Language> {
        let path = path.as_os_str().to_string_lossy();
        Self::determine_str(&path, patterns)
    }

    pub fn as_str(&self) -> &str {
        &self.name
    }

    pub fn new(lang: &str) -> Language {
        Language {
            name: lang.to_string(),
        }
    }
}
