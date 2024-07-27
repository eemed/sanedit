use std::path::Path;

use glob::{MatchOptions, Pattern};
use rustc_hash::FxHashMap;

#[derive(Debug, Hash, PartialEq, Eq, Ord, PartialOrd, Clone)]
pub struct Filetype {
    name: String,
}

impl Filetype {
    pub fn determine(path: &Path, overrides: &FxHashMap<String, Vec<String>>) -> Option<Filetype> {
        for (ft, patterns) in overrides {
            for patt in patterns {
                if let Ok(glob) = Pattern::new(patt) {
                    if glob.matches_path(path) {
                        return Some(Filetype {
                            name: ft.to_string(),
                        });
                    }
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
