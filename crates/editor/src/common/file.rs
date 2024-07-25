use std::{
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
};

use sanedit_buffer::utf8::EndOfLine;

use crate::editor::options::Options;

#[derive(Debug)]
pub(crate) struct File {
    path: PathBuf,
    eol: EndOfLine,
    size: u64,
    is_big: bool,
    read_only: bool,
}

impl File {
    pub fn new(path: impl AsRef<Path>, options: &Options) -> io::Result<File> {
        let path = path.as_ref();
        let mut file = fs::File::open(path)?;
        let metadata = file.metadata()?;
        let size = metadata.len();

        let mut buf = [0u8; 4096];
        let read = file.read(&mut buf)?;
        let eol = detect_line_ending(&buf[..read]);

        let Options {
            big_file_threshold_bytes,
            ..
        } = options;
        let is_big = *big_file_threshold_bytes <= size;
        let read_only = metadata.permissions().readonly();

        let file_metadata = File {
            path: path.into(),
            eol,
            size,
            is_big,
            read_only,
        };

        Ok(file_metadata)
    }

    pub fn read_only(&self) -> bool {
        self.read_only
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn eol(&self) -> EndOfLine {
        self.eol
    }

    pub fn size(&self) -> u64 {
        self.size
    }

    pub fn is_big(&self) -> bool {
        self.is_big
    }
}

pub(crate) fn detect_line_ending(_buf: &[u8]) -> EndOfLine {
    // TODO proper detection
    EndOfLine::default()
}
