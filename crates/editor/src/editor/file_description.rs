use std::{
    fs,
    io::{self},
    path::{Path, PathBuf},
};

use sanedit_core::Language;

use super::config::Config;

#[derive(Debug)]
pub struct FileDescription {
    pub(crate) absolute_path: PathBuf,
    pub(crate) is_big: bool,
    pub(crate) read_only: bool,
    pub(crate) language: Option<Language>,
}

impl FileDescription {
    pub fn new(path: impl AsRef<Path>, config: &Config) -> io::Result<FileDescription> {
        let path = path
            .as_ref()
            .canonicalize()
            .unwrap_or_else(|_| path.as_ref().into());
        if !path.exists() {
            return Self::new_empty(&path, config);
        }

        let file = fs::File::open(&path)?;
        let metadata = file.metadata()?;
        let size = metadata.len();

        let is_big = config.editor.big_file_threshold_bytes <= size;
        let read_only = metadata.permissions().readonly();
        let lang = Language::determine(&path, &config.editor.language_detect);

        let file_metadata = FileDescription {
            absolute_path: path.into(),
            is_big,
            read_only,
            language: lang,
        };

        Ok(file_metadata)
    }

    fn new_empty(path: &Path, config: &Config) -> io::Result<FileDescription> {
        let lang = Language::determine(path, &config.editor.language_detect);
        let file_metadata = FileDescription {
            absolute_path: path.into(),
            is_big: false,
            read_only: false, // TODO can write here?
            language: lang,
        };

        Ok(file_metadata)
    }

    pub fn language(&self) -> Option<&Language> {
        self.language.as_ref()
    }

    pub fn read_only(&self) -> bool {
        self.read_only
    }

    pub fn path(&self) -> &Path {
        &self.absolute_path
    }

    pub fn is_big(&self) -> bool {
        self.is_big
    }
}
