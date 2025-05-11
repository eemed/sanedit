use std::{
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
};

use sanedit_buffer::utf8::EndOfLine;
use sanedit_core::Filetype;

use super::config::Config;

#[derive(Debug)]
pub struct FileDescription {
    pub(crate) absolute_path: PathBuf,
    pub(crate) eol: EndOfLine,
    pub(crate) is_big: bool,
    pub(crate) read_only: bool,
    pub(crate) filetype: Option<Filetype>,
}

impl FileDescription {
    pub fn new(path: impl AsRef<Path>, config: &Config) -> io::Result<FileDescription> {
        let path = path.as_ref();
        if !path.exists() {
            return Self::new_empty(path, config);
        }

        let mut file = fs::File::open(path)?;
        let metadata = file.metadata()?;
        let size = metadata.len();

        let mut buf = [0u8; 4096];
        let read = file.read(&mut buf)?;
        let eol = detect_line_ending(&buf[..read]);

        let is_big = config.editor.big_file_threshold_bytes <= size;
        let read_only = metadata.permissions().readonly();
        let filetype = Filetype::determine(path, &config.editor.filetype_detect);

        let file_metadata = FileDescription {
            absolute_path: path.into(),
            eol,
            is_big,
            read_only,
            filetype,
        };

        Ok(file_metadata)
    }

    fn new_empty(path: &Path, config: &Config) -> io::Result<FileDescription> {
        let filetype = Filetype::determine(path, &config.editor.filetype_detect);
        let eol = filetype
            .as_ref()
            .map(|ft| config.filetype.get(ft.as_str()))
            .flatten()
            .map(|ftconfig| ftconfig.buffer.eol)
            .unwrap_or(config.editor.eol);

        let file_metadata = FileDescription {
            absolute_path: path.into(),
            eol,
            is_big: false,
            read_only: false, // TODO can write here?
            filetype,
        };

        Ok(file_metadata)
    }

    pub fn filetype(&self) -> Option<&Filetype> {
        self.filetype.as_ref()
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
