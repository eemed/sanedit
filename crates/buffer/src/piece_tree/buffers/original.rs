mod cache;
mod slice;

use std::{
    cmp::min,
    fs::File,
    io::{self, Read, Seek, SeekFrom},
    ops::Range,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, RwLock},
};

use crate::piece_tree::{buffers::ByteSlice, FILE_BACKED_MAX_PIECE_SIZE};

use self::cache::Cache;
pub(crate) use slice::OriginalBufferSlice;

#[derive(Debug)]
pub(crate) struct PathFile {
    file: File,
    path: PathBuf,
}

#[derive(Debug)]
pub(crate) enum OriginalBuffer {
    File {
        file: Mutex<PathFile>,
        cache: RwLock<Cache>,
    },
    Memory {
        bytes: Vec<u8>,
    },
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
    pub fn from_path<P: AsRef<Path>>(path: P) -> io::Result<OriginalBuffer> {
        let path = path.as_ref();
        let file = File::open(path)?;
        Ok(OriginalBuffer::File {
            file: Mutex::new(PathFile {
                file,
                path: path.into(),
            }),
            cache: RwLock::new(Cache::new()),
        })
    }

    #[inline(always)]
    pub fn slice(&self, range: Range<u64>) -> io::Result<ByteSlice<'_>> {
        use OriginalBuffer::*;
        match self {
            Memory { bytes } => Ok(bytes[range.start as usize..range.end as usize].into()),
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
                let mut pfile = file
                    .lock()
                    .map_err(|_| io::Error::from(io::ErrorKind::Other))?;

                let buf = {
                    let block = start - (start % FILE_BACKED_MAX_PIECE_SIZE as u64);
                    let size =
                        (min(len, block + FILE_BACKED_MAX_PIECE_SIZE as u64) - block) as usize;

                    let mut buf: Box<[u8]> = vec![0u8; size].into();
                    pfile.file.seek(SeekFrom::Start(block))?;
                    pfile.file.read_exact(&mut buf)?;
                    let mut buf = cache.push(block, Arc::from(buf));

                    let s = (start - block) as usize;
                    let e = (s as u64 + end - start) as usize;
                    buf.slice(s..e);
                    buf
                };

                Ok(buf.into())
            }
        }
    }

    /// Returns the length of the original buffer
    pub fn len(&self) -> u64 {
        use OriginalBuffer::*;
        match self {
            File { file, .. } => {
                if let Ok(pfile) = file.lock() {
                    pfile.file.metadata().map(|m| m.len()).unwrap_or(0)
                } else {
                    0
                }
            }
            Memory { bytes } => bytes.len() as u64,
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

    #[inline]
    pub fn file_path(&self) -> Option<PathBuf> {
        use OriginalBuffer::*;
        match self {
            File { file, .. } => {
                if let Ok(pfile) = file.lock() {
                    Some(pfile.path.clone())
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub fn rename_backing_file<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        match self {
            OriginalBuffer::File { file, .. } => {
                if let Ok(mut pfile) = file.lock() {
                    let target = path.as_ref();
                    if let Err(_) = std::fs::rename(&pfile.path, target) {
                        std::fs::copy(&pfile.path, target)?;
                        let _ = std::fs::remove_file(&pfile.path);
                    }
                    pfile.path = target.into();
                    pfile.file = File::open(target)?;
                    Ok(())
                } else {
                    unreachable!("failed to lock backing file");
                }
            }
            OriginalBuffer::Memory { .. } => {
                unreachable!("cannot rename backing file on memory buffer")
            }
        }
    }
}
