use std::{
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
};

use crate::editor::options::{Convert, EditorOptions};

use super::eol::EOL;

pub(crate) struct FileMetadata {
    pub path: PathBuf,
    pub encoding: &'static encoding_rs::Encoding,
    pub eol: EOL,
    pub size: u64,
    pub is_big: bool,
    pub convert: Convert,
}

impl FileMetadata {
    pub fn new(path: impl AsRef<Path>, options: &EditorOptions) -> io::Result<FileMetadata> {
        let path = path.as_ref();
        let mut file = fs::File::open(path)?;
        let metadata = file.metadata()?;
        let size = metadata.len();

        let mut buf = [0u8; 2048];
        let read = file.read(&mut buf)?;
        let encoding = detect_encoding(&buf[..read]);
        let eol = detect_line_ending(&buf[..read]);

        let EditorOptions {
            big_file_threshold_bytes,
            convert,
            convert_big,
            ..
        } = options;
        let is_big = *big_file_threshold_bytes <= size;
        let convert = if is_big { *convert_big } else { *convert };

        let file_metadata = FileMetadata {
            path: path.into(),
            encoding,
            eol,
            size,
            is_big,
            convert,
        };

        Ok(file_metadata)
    }

    pub fn is_utf8(&self) -> bool {
        self.encoding == encoding_rs::UTF_8
    }

    pub fn convert_to_utf8(&mut self) {
        if self.is_utf8() {
            return;
        }

        if self.is_big {
            big_file_decode_utf8();
        } else {
            file_decode_utf8();
        }
    }
}

/// Decode the file to utf8 in memory, return buffer
pub(crate) fn file_decode_utf8() {}

/// Decode the file to utf8 to a temp file, return buffer
pub(crate) fn big_file_decode_utf8() {}

/// decode a file to utf8 to a writer
pub(crate) fn decode_to_utf8() {}


pub(crate) fn detect_encoding(buf: &[u8]) -> &'static encoding_rs::Encoding {
    let mut encoding_detector = chardetng::EncodingDetector::new();
    encoding_detector.feed(buf, true);
    encoding_detector.guess(None, true)
}

pub(crate) fn detect_line_ending(buf: &[u8]) -> EOL {
    // TODO proper detection
    EOL::default()
}
