use std::{
    borrow::Cow,
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
};

use crate::editor::options::Options;

use super::{dirs::tmp_dir, eol::EOL};

#[derive(Debug)]
pub(crate) enum UTF8File {
    Memory(Vec<u8>),
    File(PathBuf),
}

#[derive(Debug)]
pub(crate) struct File {
    path: PathBuf,
    eol: EOL,
    size: u64,
    is_big: bool,
}

impl File {
    pub fn new(path: impl AsRef<Path>, options: &Options) -> io::Result<File> {
        let path = path.as_ref();
        let mut file = fs::File::open(path)?;
        let metadata = file.metadata()?;
        let size = metadata.len();

        let mut buf = [0u8; 2048];
        let read = file.read(&mut buf)?;
        let eol = detect_line_ending(&buf[..read]);

        let Options {
            big_file_threshold_bytes,
            ..
        } = options;
        let is_big = *big_file_threshold_bytes <= size;

        let file_metadata = File {
            path: path.into(),
            eol,
            size,
            is_big,
        };

        Ok(file_metadata)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn eol(&self) -> EOL {
        self.eol
    }

    pub fn size(&self) -> u64 {
        self.size
    }

    pub fn is_big(&self) -> bool {
        self.is_big
    }
}

pub(crate) fn detect_line_ending(buf: &[u8]) -> EOL {
    // TODO proper detection
    EOL::default()
}
