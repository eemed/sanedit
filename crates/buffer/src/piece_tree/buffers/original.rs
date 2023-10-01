use std::{
    fs::File,
    io,
    ops::Range,
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
};

use crate::piece_tree::buffers::ByteSlice;

#[derive(Debug)]
pub(crate) enum OriginalBuffer {
    FileBacked { map: memmap::Mmap, path: PathBuf },
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
    pub fn mmap<P: AsRef<Path>>(path: P) -> io::Result<OriginalBuffer> {
        let path = path.as_ref();
        let file = File::open(path)?;
        let map = unsafe { memmap::Mmap::map(&file)? };
        Ok(OriginalBuffer::FileBacked {
            map,
            path: path.into(),
        })
    }

    #[inline(always)]
    pub fn slice(&self, range: Range<usize>) -> ByteSlice<'_> {
        use OriginalBuffer::*;
        match self {
            Memory { bytes } => bytes[range].into(),
            FileBacked { map, .. } => map[range].into(),
        }
    }

    /// Returns the length of the original buffer
    pub fn len(&self) -> usize {
        use OriginalBuffer::*;
        match self {
            FileBacked { map, .. } => map.len(),
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

    #[inline]
    pub fn file_path(&self) -> &Path {
        use OriginalBuffer::*;
        match self {
            FileBacked { path, .. } => path,
            _ => unreachable!("no file path for memory buffer"),
        }
    }

    pub fn swap_backing_file<P: AsRef<Path>>(&self, path: P) {}
}
