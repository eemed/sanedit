use std::{
    cell::RefCell,
    cmp,
    fs::File,
    io::{self, Read, Seek, SeekFrom},
    ops::{Range, RangeBounds},
    rc::Rc,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum BufferKind {
    Add,
    Original,
}

#[derive(Debug, Clone)]
pub(crate) enum ByteSlice<'a> {
    Memory {
        bytes: &'a [u8],
    },
    File {
        start: usize,                // Start of this slice
        end: usize,                  // End of this slice
        bytes: Rc<(usize, Vec<u8>)>, // Stores a block of data read from the file
    },
}

impl<'a> ByteSlice<'a> {
    pub fn slice<R: RangeBounds<usize>>(&self, range: R) -> ByteSlice<'a> {
        let mut me = self.clone();
        let range_start = match range.start_bound() {
            std::ops::Bound::Included(i) => *i,
            std::ops::Bound::Excluded(i) => *i + 1,
            std::ops::Bound::Unbounded => 0,
        };

        let range_end = match range.end_bound() {
            std::ops::Bound::Included(i) => *i + 1,
            std::ops::Bound::Excluded(i) => *i,
            std::ops::Bound::Unbounded => self.as_ref().len(),
        };

        match &mut me {
            ByteSlice::Memory { bytes } => {
                *bytes = &bytes[range_start..range_end];
            }
            ByteSlice::File { start, end, bytes } => {
                *start = range_start;
                *end = range_end;

                debug_assert!(bytes.1.len() < *end);
            }
        }

        me
    }
}

impl<'a> AsRef<[u8]> for ByteSlice<'a> {
    #[inline(always)]
    fn as_ref(&self) -> &[u8] {
        match self {
            ByteSlice::Memory { bytes } => bytes,
            ByteSlice::File { start, end, bytes } => &bytes.1[*start..*end],
        }
    }
}

impl<'a> PartialEq for ByteSlice<'a> {
    fn eq(&self, other: &Self) -> bool {
        let bytes = self.as_ref();
        let other = other.as_ref();
        bytes.eq(other)
    }
}

impl<'a> From<&'a [u8]> for ByteSlice<'a> {
    fn from(bytes: &'a [u8]) -> Self {
        ByteSlice::Memory { bytes }
    }
}

pub(crate) type AddBuffer = Vec<u8>;

#[derive(Debug)]
pub(crate) enum OriginalBuffer {
    // Uses a backing file to read the data from. The file data is read in
    // blocks and cached. The Rc pointer is cloned to the slices given out, so
    // we can change the cache here and the slice will hold the block alive.
    File {
        file: RefCell<File>,                  // File handle to read data from
        cache: RefCell<Rc<(usize, Vec<u8>)>>, // Stores a block of data read from the file
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

    pub fn from_file(_file: File) -> io::Result<OriginalBuffer> {
        todo!()
    }

    #[inline(always)]
    pub fn slice(&self, range: Range<usize>) -> io::Result<ByteSlice<'_>> {
        use OriginalBuffer::*;
        let Range { start, end } = range;

        match self {
            Memory { bytes } => {
                let bytes = ByteSlice::Memory {
                    bytes: &bytes[range],
                };
                Ok(bytes)
            }
            File { cache, .. } => {
                {
                    let c = cache.borrow();
                    let (cache_pos, cache_bytes) = c.as_ref();

                    if *cache_pos <= start && end <= *cache_pos + cache_bytes.len() {
                        let bytes = ByteSlice::File {
                            start: start - cache_pos,
                            end: end - cache_pos,
                            bytes: c.clone(),
                        };
                        return Ok(bytes);
                    }
                }

                self.read_from_file(start, end)
            }
        }
    }

    #[inline]
    fn read_from_file(&self, start: usize, end: usize) -> io::Result<ByteSlice<'_>> {
        const MIN_BUFFER_SIZE: usize = 1024 * 1024;

        match self {
            OriginalBuffer::File { file, cache } => {
                let mut file = file.borrow_mut();
                // Read at minimum MIN_BUFFER_SIZE
                let read_len = cmp::max(MIN_BUFFER_SIZE, end - start);
                let read_pos = {
                    // Balance the read so that requested chunk is in the middle
                    // p
                    // |---------------------------|
                    //      diff      |------------|
                    //                start        end
                    let p = end.saturating_sub(read_len);
                    let diff = start - p;

                    p + diff / 2
                };
                // let file_len = file.metadata()?.len() as usize;

                file.seek(SeekFrom::Start(read_pos as u64))?;
                let mut bytes = Vec::with_capacity(read_len);
                let read = file
                    .by_ref()
                    .take(read_len as u64)
                    .read_to_end(&mut bytes)?;
                // if read == 0 {
                // }

                let chunk = Rc::new((read_pos, bytes));
                let mut c = cache.borrow_mut();
                *c = chunk.clone();

                let bytes = ByteSlice::File {
                    start: read_pos,
                    end: read_pos + read,
                    bytes: chunk,
                };
                Ok(bytes)
            }
            _ => unreachable!(),
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
