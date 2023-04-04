use std::{
    borrow::Cow,
    cell::RefCell,
    fs::File,
    io::{self, Read, Seek, SeekFrom},
    ops::Range,
};

use crate::piece_tree::FILE_BACKED_MAX_PIECE_SIZE;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum BufferKind {
    Add,
    Original,
}

pub(crate) type ByteSlice<'a> = Cow<'a, [u8]>;
pub(crate) type AddBuffer = Vec<u8>;

#[derive(Debug)]
pub(crate) struct Cache {
    cache: Box<[u8]>,
    /// List of pointers to cache. (cache_offset, buf_offset, length) tuples
    cache_ptrs: Vec<(usize, usize, usize)>,
    next: usize,
}

impl Cache {
    const FILE_CACHE_SIZE: usize = FILE_BACKED_MAX_PIECE_SIZE * 10;

    pub fn new() -> Cache {
        Cache {
            cache: [0u8; Self::FILE_CACHE_SIZE].into(),
            cache_ptrs: Vec::new(),
            next: 0,
        }
    }

    fn get(&self, start: usize, end: usize) -> Option<&[u8]> {
        for (pos, bpos, len) in &self.cache_ptrs {
            if *bpos <= start && end <= bpos + len {
                return Some(self.cache[*pos..pos + end - start].into());
            }
        }

        None
    }

    fn find_space_for(&mut self, bpos: usize, len: usize) -> &mut [u8] {
        let mut start = self.next;
        let mut end = start + len;
        if Self::FILE_CACHE_SIZE < end {
            start = 0;
            end = len;
        }
        self.next = end;

        self.cache_ptrs.retain(|(s, _, l)| {
            let e = s + l;
            !(start <= e && *s <= end)
        });
        self.cache_ptrs.push((start, bpos, len));

        &mut self.cache[start..end]
    }
}

#[derive(Debug)]
pub(crate) enum OriginalBuffer {
    // Uses a backing file to read the data from. The file data is read in
    // blocks and cached.
    File {
        file: RefCell<File>, // File handle to read data from
        cache: RefCell<Cache>,
    },
    Memory {
        bytes: Vec<u8>,
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

    pub fn from_file(file: File) -> OriginalBuffer {
        OriginalBuffer::File {
            file: RefCell::new(file),
            cache: RefCell::new(Cache::new()),
        }
    }

    #[inline(always)]
    pub fn slice(&self, range: Range<usize>) -> io::Result<ByteSlice<'_>> {
        use OriginalBuffer::*;
        match self {
            Memory { bytes } => Ok(bytes[range].into()),
            File { cache, file } => {
                let Range { start, end } = range;
                if let Some(bytes) = cache.borrow().get(start, end) {
                    return Ok(bytes.to_vec().into());
                }

                let mut cache = cache.borrow_mut();
                let mut file = file.borrow_mut();

                let buf = cache.find_space_for(start, range.len());
                file.seek(SeekFrom::Start(start as u64))?;
                file.read_exact(buf)?;

                Ok(buf.to_vec().into())
            }
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        use OriginalBuffer::*;
        match self {
            File { file, .. } => file.borrow().metadata().map(|m| m.len()).unwrap_or(0) as usize,
            Memory { bytes } => bytes.len(),
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
