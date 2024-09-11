use std::{
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
};

use rustc_hash::FxHashMap;
use sanedit_buffer::utf8::EndOfLine;

use crate::Filetype;

#[derive(Debug)]
pub struct FileDescription {
    absolute_path: PathBuf,
    local_path: PathBuf,
    eol: EndOfLine,
    size: u64,
    is_big: bool,
    read_only: bool,
    filetype: Option<Filetype>,
}

impl FileDescription {
    pub fn new(
        path: impl AsRef<Path>,
        big_file_threshold_bytes: u64,
        working_dir: &Path,
        filetype_patterns: &FxHashMap<String, Vec<String>>,
    ) -> io::Result<FileDescription> {
        let path = path.as_ref();
        let mut file = fs::File::open(path)?;
        let metadata = file.metadata()?;
        let size = metadata.len();

        let mut buf = [0u8; 4096];
        let read = file.read(&mut buf)?;
        let eol = detect_line_ending(&buf[..read]);

        let is_big = big_file_threshold_bytes <= size;
        let read_only = metadata.permissions().readonly();
        let local = path.strip_prefix(working_dir).unwrap_or(path);
        let filetype = Filetype::determine(path, filetype_patterns);

        let file_metadata = FileDescription {
            absolute_path: path.into(),
            local_path: local.into(),
            eol,
            size,
            is_big,
            read_only,
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

    pub fn local_path(&self) -> &Path {
        &self.local_path
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
