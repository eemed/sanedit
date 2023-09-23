use crate::piece_tree::FILE_BACKED_MAX_PIECE_SIZE;
use std::{
    cmp,
    collections::VecDeque,
    fs::File,
    io::{self, Read, Seek, SeekFrom},
    ops::Range,
    sync::{Arc, Mutex, RwLock},
};

use super::ByteSlice;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OriginalBufferSlice {
    ptr: Arc<[u8]>,
    offset: usize,
    len: usize,
}

impl AsRef<[u8]> for OriginalBufferSlice {
    fn as_ref(&self) -> &[u8] {
        &self.ptr[self.offset..self.offset + self.len]
    }
}

impl OriginalBufferSlice {
    pub fn slice(&mut self, range: Range<usize>) {
        self.offset += range.start;
        self.len = range.len();
    }
}

#[derive(Debug)]
pub(crate) struct Cache {
    /// List of pointers to cache. (buf_offset, length) tuples
    cache_ptrs: VecDeque<(usize, Arc<[u8]>)>,
}

impl Cache {
    const CACHE_SIZE: usize = 10;

    pub fn new() -> Cache {
        Cache {
            cache_ptrs: VecDeque::new(),
        }
    }

    fn get(&self, start: usize, end: usize) -> Option<OriginalBufferSlice> {
        for (off, ptr) in &self.cache_ptrs {
            if *off <= start && end <= off + ptr.len() {
                let s = start - off;
                let e = s + end - start;
                return Some(OriginalBufferSlice {
                    ptr: ptr.clone(),
                    offset: s,
                    len: e - s,
                });
            }
        }

        None
    }

    fn push(&mut self, off: usize, ptr: Arc<[u8]>) -> OriginalBufferSlice {
        while self.cache_ptrs.len() >= Self::CACHE_SIZE {
            self.cache_ptrs.pop_front();
        }

        self.cache_ptrs.push_back((off, ptr.clone()));

        OriginalBufferSlice {
            offset: 0,
            len: ptr.len(),
            ptr,
        }
    }
}

#[derive(Debug)]
pub(crate) enum OriginalBuffer {
    // Uses a backing file to read the data from. The file data is read in
    // blocks and cached.
    File {
        file: Mutex<File>, // File handle to read data from
        cache: RwLock<Cache>,
    },
    Memory {
        bytes: Vec<u8>,
    },
    Mmap {
        map: memmap::Mmap,
    },
}

impl OriginalBuffer {
    #[inline]
    pub fn new() -> OriginalBuffer {
        OriginalBuffer::Memory { bytes: vec![] }
    }

    #[inline]
    pub fn from_reader<T: io::Read>(mut reader: T) -> io::Result<OriginalBuffer> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes)?;
        Ok(OriginalBuffer::Memory { bytes })
    }

    #[inline]
    pub fn from_file(file: File) -> OriginalBuffer {
        OriginalBuffer::File {
            file: Mutex::new(file),
            cache: RwLock::new(Cache::new()),
        }
    }

    #[inline]
    pub fn mmap(file: File) -> io::Result<OriginalBuffer> {
        let mmap = unsafe { memmap::Mmap::map(&file)? };
        Ok(OriginalBuffer::Mmap { map: mmap })
    }

    #[inline(always)]
    pub fn slice(&self, range: Range<usize>) -> io::Result<ByteSlice<'_>> {
        use OriginalBuffer::*;
        match self {
            Memory { bytes } => Ok(bytes[range].into()),
            Mmap { map } => Ok(map[range].into()),
            File { cache, file } => {
                let Range { start, end } = range;
                {
                    let ro_cache = cache
                        .read()
                        .map_err(|_| io::Error::from(io::ErrorKind::Other))?;
                    if let Some(slice) = ro_cache.get(start, end) {
                        return Ok(slice.into());
                    }
                }

                let len = self.len();
                let mut cache = cache
                    .write()
                    .map_err(|_| io::Error::from(io::ErrorKind::Other))?;
                let mut file = file
                    .lock()
                    .map_err(|_| io::Error::from(io::ErrorKind::Other))?;

                let buf = {
                    let block = start - (start % FILE_BACKED_MAX_PIECE_SIZE);
                    let size = cmp::min(len, block + FILE_BACKED_MAX_PIECE_SIZE) - block;

                    let mut buf: Box<[u8]> = vec![0u8; size].into();
                    file.seek(SeekFrom::Start(block as u64))?;
                    file.read_exact(&mut buf)?;
                    let mut buf = cache.push(block, Arc::from(buf));

                    let s = start - block;
                    let e = s + end - start;
                    buf.slice(s..e);
                    buf
                };

                Ok(buf.into())
            }
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        use OriginalBuffer::*;
        match self {
            File { file, .. } => {
                if let Ok(file) = file.lock() {
                    file.metadata().map(|m| m.len()).unwrap_or(0) as usize
                } else {
                    0
                }
            }
            Memory { bytes } => bytes.len(),
            Mmap { map } => map.len(),
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    pub fn is_file_backed(&self) -> bool {
        matches!(self, OriginalBuffer::File { .. })
    }
}
