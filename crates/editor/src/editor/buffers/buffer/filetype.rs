use std::path::Path;

#[derive(Debug, PartialEq, Eq, Ord, PartialOrd, Clone)]
pub struct Filetype {
    name: String,
}

impl Filetype {
    pub fn determine(path: &Path) -> Option<Filetype> {
        // match filename {
        // }
        //
        let ext = path.extension()?;
        let ftype = ext.to_string_lossy();

        Some(Filetype { name: ftype.into() })
    }

    pub fn as_str(&self) -> &str {
        &self.name
    }
}
