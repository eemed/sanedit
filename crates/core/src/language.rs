use std::{
    io::Read as _,
    path::Path,
    sync::{Arc, OnceLock},
};

use rustc_hash::FxHashMap;
use sanedit_syntax::GitGlob;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct Detect {
    extensions: Vec<String>,
    globs: Vec<String>,
    shebangs: Vec<String>,

    #[serde(skip)]
    compiled_globs: Arc<OnceLock<Vec<GitGlob>>>,
}

impl Detect {
    pub fn new(extensions: Vec<String>, globs: Vec<String>, shebangs: Vec<String>) -> Detect {
        Detect {
            extensions,
            globs,
            shebangs,
            compiled_globs: Arc::new(OnceLock::new()),
        }
    }

    pub fn compile_globs(&self) -> &Vec<GitGlob> {
        self.compiled_globs.get_or_init(move || {
            let mut globs = Vec::with_capacity(self.globs.len());
            for pat in &self.globs {
                if let Ok(glob) = GitGlob::new(pat) {
                    globs.push(glob);
                }
            }
            globs
        })
    }

    pub fn merge(&mut self, detect: Detect) {
        self.extensions.extend(detect.extensions);
        self.extensions.dedup();

        self.globs.extend(detect.globs);
        self.globs.dedup();

        self.shebangs.extend(detect.shebangs);
        self.shebangs.dedup();

        self.compiled_globs = Arc::new(OnceLock::new());
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
    pub fn determine<P: AsRef<Path>>(
        pattern: P,
        patterns: &FxHashMap<String, Detect>,
    ) -> Option<Language> {
        let path = pattern.as_ref();
        let pattern = path.as_os_str().to_string_lossy();
        let mut buf = [0u8; 128];

        for (ft, detect) in patterns {
            if ft.as_str() == pattern {
                return Some(Language {
                    name: ft.to_string(),
                });
            }

            let ext = path
                .extension()
                .and_then(std::ffi::OsStr::to_str)
                .unwrap_or(pattern.as_ref());
            for dext in &detect.extensions {
                if ext == dext {
                    return Some(Language {
                        name: ft.to_string(),
                    });
                }
            }

            for glob in detect.compile_globs() {
                if glob.is_match(&pattern.as_bytes()) {
                    return Some(Language {
                        name: ft.to_string(),
                    });
                }
            }

            let n = std::fs::File::open(path)
                .ok()
                .and_then(|mut file| file.read(&mut buf).ok());
            if let Some(n) = n {
                let read = &buf[..n];
                for shebang in &detect.shebangs {
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

        Some(Language {
            name: pattern.into(),
        })
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
