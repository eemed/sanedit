use std::{fs::File, io, ops::Range};

use crate::piece_tree::buffers::ByteSlice;

#[derive(Debug)]
pub(crate) enum OriginalBuffer {
    FileBacked { map: memmap::Mmap },
    Memory { bytes: Vec<u8> },
}

impl OriginalBuffer {
    #[inline]
    pub fn new() -> OriginalBuffer {
        OriginalBuffer::Memory { bytes: Vec::new() }
    }

    #[inline]
    pub fn from_reader<T: io::Read>(mut reader: T) -> io::Result<OriginalBuffer> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes)?;
        Ok(OriginalBuffer::Memory { bytes })
    }

    #[inline]
    pub fn mmap(file: File) -> io::Result<OriginalBuffer> {
        let map = unsafe { memmap::Mmap::map(&file)? };
        Ok(OriginalBuffer::FileBacked { map })
    }

    #[inline(always)]
    pub fn slice(&self, range: Range<usize>) -> ByteSlice<'_> {
        use OriginalBuffer::*;
        match self {
            Memory { bytes } => bytes[range].into(),
            FileBacked { map } => map[range].into(),
        }
    }

    /// Returns the length of the original buffer
    pub fn len(&self) -> usize {
        use OriginalBuffer::*;
        match self {
            FileBacked { map } => map.len(),
            Memory { bytes } => bytes.len(),
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    pub fn is_file_backed(&self) -> bool {
        use OriginalBuffer::*;
        match self {
            FileBacked { .. } => true,
            _ => false,
        }
    }
}
