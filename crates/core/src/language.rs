use std::{io::Read as _, path::Path};

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

        if let Some(lang) = Self::determine_shebang(path) {
            return lang.into();
        }

        let ext = path.extension()?;
        let lang = ext.to_string_lossy();
        Some(Language { name: lang.into() })
    }

    fn determine_shebang(path: &Path) -> Option<Language> {
        const SHEBANG: &[u8] = b"#!";
        const ENV: &[u8] = b"/usr/bin/env";
        const BIN_BASH: &[u8] = b"/bin/bash";

        let file = std::fs::File::open(path).ok()?;
        let mut reader = std::io::BufReader::new(file);
        let mut buf = vec![0u8; 1024];
        let n = reader.read(&mut buf).ok()?;
        buf.truncate(n);
        if !buf.starts_with(SHEBANG) {
            return None;
        }

        let rest = &buf[SHEBANG.len()..];
        if rest.starts_with(ENV) {
            let nl = rest
                .iter()
                .position(|b| *b == '\n' as u8 || *b == '\r' as u8)
                .unwrap_or(rest.len());
            let interpreter = &rest[ENV.len()..nl];
            match interpreter {
                b"bash" | b"sh" | b"zsh" | b"dash" | b"ksh" => {
                    return Language::from("shellscript").into()
                }
                b"node" | b"nodejs" => return Language::from("javascript").into(),
                b"python3" => return Language::from("python").into(),
                _ => {}
            }
        }
        if rest.starts_with(BIN_BASH) {
            return Language::from("shellscript").into();
        }

        None
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
