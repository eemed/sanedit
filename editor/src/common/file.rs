use std::{
    fs::{self, File},
    io::{self, Read},
    path::{Path, PathBuf},
};

use crate::editor::{buffers::Buffer, options::EditorOptions};

use super::eol::EOL;

pub(crate) struct FileMetadata {
    pub path: PathBuf,
    pub encoding: &'static encoding_rs::Encoding,
    pub eol: EOL,
    pub size: u64,
}

impl TryFrom<&Path> for FileMetadata {
    type Error = io::Error;

    fn try_from(path: &Path) -> Result<Self, Self::Error> {
        let file = fs::File::open(path)?;
        let metadata = file.metadata()?;
        let size = metadata.len();

        let mut buf = [0u8; 2048];
        let read = file.read(&mut buf)?;

        let encoding = detect_encoding(&buf[..read]);
        let eol = detect_line_ending(&buf[..read]);

        let file_metadata = FileMetadata {
            path: path.into(),
            encoding,
            eol,
            size,
        };

        Ok(file_metadata)
    }
}

pub(crate) fn detect_encoding(buf: &[u8]) -> &'static encoding_rs::Encoding {
    let mut encoding_detector = chardetng::EncodingDetector::new();
    encoding_detector.feed(buf, true);
    encoding_detector.guess(None, true)
}

pub(crate) fn detect_line_ending(buf: &[u8]) -> EOL {
    todo!()
}

/// Decode the file to utf8 in memory, return buffer
pub(crate) fn file_decode_utf8() {}

/// Decode the file to utf8 to a temp file, return buffer
pub(crate) fn big_file_decode_utf8() {}

/// decode a file to utf8 to a writer
pub(crate) fn decode_to_utf8() {}
