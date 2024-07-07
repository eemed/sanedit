use std::path::Path;

#[derive(Debug, Hash, PartialEq, Eq, Ord, PartialOrd, Clone)]
pub struct Filetype {
    name: String,
}

impl Filetype {
    pub fn determine(path: &Path) -> Option<Filetype> {
        let fname = path.file_name()?;
        let fname = fname.to_string_lossy();
        let ftype = match fname.as_ref() {
            "Cargo.lock" => "toml".into(),
            _ => {
                let ext = path.extension()?;
                match ext.to_str() {
                    Some("rs") => "rust".into(),
                    _ => ext.to_string_lossy(),
                }
            }
        };

        Some(Filetype { name: ftype.into() })
    }

    pub fn as_str(&self) -> &str {
        &self.name
    }
}
