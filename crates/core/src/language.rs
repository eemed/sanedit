use std::path::Path;

use rustc_hash::FxHashMap;
use sanedit_syntax::Glob;

#[derive(Debug, Hash, PartialEq, Eq, Ord, PartialOrd, Clone)]
pub struct Language {
    name: String,
}

impl Language {
    pub fn determine(path: &Path, patterns: &FxHashMap<String, Vec<String>>) -> Option<Language> {
        for (ft, patterns) in patterns {
            let mut globs = vec![];
            patterns.iter().for_each(|pat| {
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
            v => v,
        };
        Language {
            name: name.to_string(),
        }
    }
}
