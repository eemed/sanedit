use std::{
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
};

use crate::editor::options::{Convert, EditorOptions};

use super::eol::EOL;

#[derive(Debug)]
pub(crate) enum UTF8File {
    Memory(Vec<u8>),
    File(PathBuf),
}

#[derive(Debug)]
pub(crate) struct File {
    path: PathBuf,
    encoding: &'static encoding_rs::Encoding,
    eol: EOL,
    size: u64,
    is_big: bool,
    convert: Convert,

    /// If this file was converted to UTF-8, stores the data.
    pub converted: Option<UTF8File>,
}

impl File {
    pub fn new(path: impl AsRef<Path>, options: &EditorOptions) -> io::Result<File> {
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

        let file_metadata = File {
            path: path.into(),
            encoding,
            eol,
            size,
            is_big,
            convert,
            converted: None,
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

    pub fn convert(&self) -> Convert {
        self.convert
    }

    pub fn is_big(&self) -> bool {
        self.is_big
    }

    pub fn encoding(&self) -> &'static encoding_rs::Encoding {
        self.encoding
    }

    pub fn is_utf8(&self) -> bool {
        self.encoding == encoding_rs::UTF_8
    }

    pub fn is_converted(&self) -> bool {
        self.converted.is_some()
    }

    /// Decodes the file to UTF-8. Big files are converted to a temp file and
    /// small files are converted in memory.
    pub fn decode_to_utf8(&mut self) -> io::Result<()> {
        if self.is_utf8() || self.is_converted() {
            return Ok(());
        }

        if self.is_big {
            // TODO get temp file dir
            // self.decode_to_utf8_file()
            Ok(())
        } else {
            self.decode_to_utf8_vec()
        }
    }

    /// Decode the file to utf8 to a temp file
    pub fn decode_to_utf8_file(&mut self, to: impl AsRef<Path>) -> io::Result<()> {
        if self.is_utf8() || self.is_converted() {
            return Ok(());
        }

        let path = to.as_ref().to_path_buf();
        let mut input = fs::File::open(&self.path)?;
        let mut output = fs::File::create(&path)?;
        let (read, written) = decode_to_utf8(input, self.encoding, &mut output)?;
        if read as u64 != self.size {
            todo!("Failed to decode to file");
        }

        self.converted = Some(UTF8File::File(path));

        Ok(())
    }

    /// Decode the file to utf8 in memory, return buffer
    pub fn decode_to_utf8_vec(&mut self) -> io::Result<()> {
        if self.is_utf8() || self.is_converted() {
            return Ok(());
        }

        let mut input = fs::File::open(&self.path)?;
        let mut output = Vec::new();
        let (read, written) = decode_to_utf8(input, self.encoding, &mut output)?;
        if read as u64 != self.size {
            todo!("Failed to decode to buffer");
        }

        self.converted = Some(UTF8File::Memory(output));
        Ok(())
    }
}

/// decode a file to utf8 to a writer
pub(crate) fn decode_to_utf8<R: io::Read, W: io::Write>(
    mut reader: R,
    encoding: &'static encoding_rs::Encoding,
    writer: &mut W,
) -> io::Result<(usize, usize)> {
    const BUFFER_SIZE: usize = 4096;
    // Buffers to read input
    let mut buf: [u8; BUFFER_SIZE] = [0; BUFFER_SIZE];
    let mut buf_read = reader.read(&mut buf)?;

    // Buffers to decode to
    let mut buf_out: [u8; BUFFER_SIZE] = [0; BUFFER_SIZE];
    let buf_str = unsafe { std::str::from_utf8_unchecked_mut(&mut buf_out) };

    // Wether input can be read more
    let mut is_last = buf_read == 0;
    // keep track of decoder read and written bytes to buffer
    let mut buf_total_written: usize = 0;
    let mut buf_total_read: usize = 0;

    // Totals read from the reader and written to the writer
    let mut total_written: usize = 0;
    let mut total_read: usize = 0;

    // Get decoder
    let mut decoder = encoding.new_decoder();

    loop {
        let (result, read, written, _) = decoder.decode_to_str(
            &buf[buf_total_read..buf_read],
            &mut buf_str[buf_total_written..],
            is_last,
        );
        buf_total_read += read;
        total_read += read;
        buf_total_written += written;
        total_written += written;

        match result {
            encoding_rs::CoderResult::InputEmpty => {
                if is_last {
                    writer.write(buf_str[..buf_total_written].as_bytes())?;
                    break;
                }
            }
            encoding_rs::CoderResult::OutputFull => {
                writer.write(buf_str[..buf_total_written].as_bytes())?;
                buf_total_written = 0;
            }
        }

        // If everything is decoded read more input
        if buf_total_read == buf_read {
            buf_read = reader.read(&mut buf)?;
            is_last = buf_read == 0;
            buf_total_read = 0;
        }
    }

    Ok((total_read, total_written))
}

pub(crate) fn detect_encoding(buf: &[u8]) -> &'static encoding_rs::Encoding {
    let mut encoding_detector = chardetng::EncodingDetector::new();
    encoding_detector.feed(buf, true);
    encoding_detector.guess(None, true)
}

pub(crate) fn detect_line_ending(buf: &[u8]) -> EOL {
    // TODO proper detection
    EOL::default()
}
