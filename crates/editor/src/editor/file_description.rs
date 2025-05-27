use std::{
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
};

use sanedit_buffer::utf8::EndOfLine;
use sanedit_core::Language;

use super::{config::Config, language::Languages};

#[derive(Debug)]
pub struct FileDescription {
    pub(crate) absolute_path: PathBuf,
    pub(crate) eol: EndOfLine,
    pub(crate) is_big: bool,
    pub(crate) read_only: bool,
    pub(crate) language: Option<Language>,
}

impl FileDescription {
    pub fn new(
        path: impl AsRef<Path>,
        config: &Config,
        langs: &Languages,
    ) -> io::Result<FileDescription> {
        let path = path.as_ref();
        if !path.exists() {
            return Self::new_empty(path, config, langs);
        }

        let mut file = fs::File::open(path)?;
        let metadata = file.metadata()?;
        let size = metadata.len();

        let mut buf = [0u8; 4096];
        let read = file.read(&mut buf)?;
        let eol = detect_line_ending(&buf[..read]);

        let is_big = config.editor.big_file_threshold_bytes <= size;
        let read_only = metadata.permissions().readonly();
        let lang = Language::determine(path, &config.editor.language_detect);

        let file_metadata = FileDescription {
            absolute_path: path.into(),
            eol,
            is_big,
            read_only,
            language: lang,
        };

        Ok(file_metadata)
    }

    fn new_empty(
        path: &Path,
        config: &Config,
        langs: &Languages,
    ) -> io::Result<FileDescription> {
        let lang = Language::determine(path, &config.editor.language_detect);
        let eol = lang
            .as_ref()
            .map(|lang| langs.get(&lang))
            .flatten()
            .map(|langconfig| langconfig.buffer.eol)
            .unwrap_or(config.editor.eol);

        let file_metadata = FileDescription {
            absolute_path: path.into(),
            eol,
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

    #[allow(dead_code)]
    pub fn eol(&self) -> EndOfLine {
        self.eol
    }

    pub fn is_big(&self) -> bool {
        self.is_big
    }
}

pub(crate) fn detect_line_ending(_buf: &[u8]) -> EndOfLine {
    // TODO proper detection
    EndOfLine::default()
}
