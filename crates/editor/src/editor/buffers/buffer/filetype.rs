use std::path::Path;

use globset::{Glob, GlobSetBuilder};
use rustc_hash::FxHashMap;

#[derive(Debug, Hash, PartialEq, Eq, Ord, PartialOrd, Clone)]
pub struct Filetype {
    name: String,
}

impl Filetype {
    pub fn determine(path: &Path, overrides: &FxHashMap<String, Vec<String>>) -> Option<Filetype> {
        for (ft, patterns) in overrides {
            let mut builder = GlobSetBuilder::new();
            patterns.iter().for_each(|pat| {
                if let Ok(glob) = Glob::new(pat) {
                    builder.add(glob);
                }
            });

            if let Ok(glob) = builder.build() {
                if !glob.matches(path).is_empty() {
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
