use std::{io::Read as _, path::Path};

use rustc_hash::FxHashMap;
use sanedit_syntax::Glob;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Detect {
    glob: Vec<String>,
    shebang: Vec<String>,
}

impl Detect {
    pub fn new(globs: Vec<String>, shebangs: Vec<String>) -> Detect {
        Detect {
            glob: globs,
            shebang: shebangs,
        }
    }

    pub fn merge(&mut self, detect: Detect) {
        self.glob.extend(detect.glob);
        self.glob.dedup();
        self.shebang.extend(detect.shebang);
        self.shebang.dedup();
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Ord, PartialOrd, Clone)]
pub struct Language {
    name: String,
}

impl Language {
    pub fn determine(path: &Path, patterns: &FxHashMap<String, Detect>) -> Option<Language> {
        let mut buf = [0u8; 128];

        for (ft, detect) in patterns {
            let mut globs = vec![];
            detect.glob.iter().for_each(|pat| {
                if let Ok(glob) = Glob::new(pat) {
                    globs.push(glob);
                }
            });

            for glob in globs {
                let path = path.as_os_str().to_string_lossy();
                if glob.is_match(&path.as_bytes()) {
                    return Some(Language {
                        name: ft.to_string(),
                    });
                }
            }

            let n = std::fs::File::open(path)
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

        let ext = path.extension()?;
        let lang = ext.to_string_lossy();
        Some(Language { name: lang.into() })
    }

    pub fn as_str(&self) -> &str {
        &self.name
    }
}

impl From<&str> for Language {
    fn from(value: &str) -> Self {
        // Transform common names to LSP language identifiers
        let name = match value {
            "tsx" => "typescriptreact",
            "jsx" => "javascriptreact",
            "js" => "javascript",
            "ts" => "typescript",
            "sh" => "shellscript",
            "bash" => "shellscript",
            v => v,
        };
        Language {
            name: name.to_string(),
        }
    }
}
