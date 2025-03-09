use std::path::Path;

use rustc_hash::FxHashMap;
use sanedit_syntax::Glob;

#[derive(Debug, Hash, PartialEq, Eq, Ord, PartialOrd, Clone)]
pub struct Filetype {
    name: String,
}

impl Filetype {
    pub fn determine(path: &Path, patterns: &FxHashMap<String, Vec<String>>) -> Option<Filetype> {
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
                    return Some(Filetype {
                        name: ft.to_string(),
                    });
                }
            }
        }

        let ext = path.extension()?;
        let ftype = ext.to_string_lossy();
        Some(Filetype { name: ftype.into() })
    }

    pub fn as_str(&self) -> &str {
        &self.name
    }
}
