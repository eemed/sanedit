use std::{
    borrow::Cow,
    cmp::min,
    fs::File,
    io::{self, Read, Seek},
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use sanedit_buffer::{utf8::decode_utf8_iter, Bytes, Chunk, Chunks, PieceTreeSlice};

pub trait ByteSource {
    /// If ByteSource is contiguous in memory, returns it as a single chunk
    fn as_single_chunk(&mut self) -> Option<&[u8]>;

    /// Used if ByteSource is not contiguous in memory and needs to be copied to a sliding window.
    /// a shitty way to copy. Override to provide a better alternative if possible. this is slow
    fn copy_to(&mut self, at: u64, buf: &mut [u8]) -> usize {
        debug_assert!(
            self.as_single_chunk().is_none(),
            "Copying a contiguous ByteSource"
        );

        let start = at;
        let end = std::cmp::min(self.len(), at + buf.len() as u64);

        for (buf_index, byte_index) in (start..end).enumerate() {
            buf[buf_index] = self.get(byte_index);
        }

        (end.saturating_sub(start)) as usize
    }

    /// Length of all the bytes in this reader
    fn len(&self) -> u64;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Wether to stop parsing and return an error
    fn stop(&self) -> bool;

    fn get(&mut self, at: u64) -> u8;

    fn char_between(&mut self, at: u64, start: char, end: char) -> Option<u64> {
        let max = min(4, self.len() - at);
        let mut bytes = [0u8; 4];
        for i in 0..max {
            bytes[i as usize] = self.get(at + i);
        }
        let (ch, size) = decode_utf8_iter(bytes[..max as usize].iter().copied());
        let ch = ch?;

        if start <= ch && ch <= end {
            Some(size)
        } else {
            None
        }
    }
}

impl ByteSource for &str {
    fn len(&self) -> u64 {
        <str>::len(self) as u64
    }

    fn stop(&self) -> bool {
        false
    }

    fn get(&mut self, at: u64) -> u8 {
        self.as_bytes()[at as usize]
    }

    fn as_single_chunk(&mut self) -> Option<&[u8]> {
        Some(self.as_bytes())
    }
}

impl ByteSource for &[u8] {
    fn len(&self) -> u64 {
        <[u8]>::len(self) as u64
    }

    fn stop(&self) -> bool {
        false
    }

    fn get(&mut self, at: u64) -> u8 {
        self[at as usize]
    }

    fn as_single_chunk(&mut self) -> Option<&[u8]> {
        Some(self)
    }
}

impl<B: ByteSource> ByteSource for (B, Arc<AtomicBool>) {
    fn len(&self) -> u64 {
        self.0.len()
    }

    fn stop(&self) -> bool {
        self.1.load(Ordering::Acquire)
    }

    fn get(&mut self, at: u64) -> u8 {
        self.0.get(at)
    }

    fn as_single_chunk(&mut self) -> Option<&[u8]> {
        self.0.as_single_chunk()
    }
}

impl<const N: usize> ByteSource for &[u8; N] {
    #[inline(always)]
    fn len(&self) -> u64 {
        N as u64
    }

    #[inline(always)]
    fn get(&mut self, i: u64) -> u8 {
        self[i as usize]
    }

    fn stop(&self) -> bool {
        false
    }

    fn as_single_chunk(&mut self) -> Option<&[u8]> {
        Some(*self)
    }
}

impl<'a> ByteSource for Cow<'a, [u8]> {
    fn len(&self) -> u64 {
        let r = self.as_ref();
        r.len() as u64
    }

    fn get(&mut self, i: u64) -> u8 {
        self[i as usize]
    }

    fn stop(&self) -> bool {
        false
    }

    fn as_single_chunk(&mut self) -> Option<&[u8]> {
        Some(self)
    }
}

#[derive(Debug)]
pub struct PTSliceSource<'a> {
    slice: &'a PieceTreeSlice,
    bytes: Bytes<'a>,
    chunks: Chunks<'a>,
    chunk: Option<Chunk<'a>>,
    pub stop: Arc<AtomicBool>,
}

impl<'a> PTSliceSource<'a> {
    pub fn new(slice: &PieceTreeSlice) -> PTSliceSource<'_> {
        let bytes = slice.bytes();
        let chunks = slice.chunks();
        PTSliceSource {
            slice,
            bytes,
            chunks,
            chunk: None,
            stop: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl<'a> From<&'a PieceTreeSlice> for PTSliceSource<'a> {
    fn from(value: &'a PieceTreeSlice) -> Self {
        PTSliceSource::new(value)
    }
}

impl<'a> ByteSource for PTSliceSource<'a> {
    fn len(&self) -> u64 {
        <Bytes>::len(&self.bytes)
    }

    fn stop(&self) -> bool {
        self.stop.load(Ordering::Acquire)
    }

    fn get(&mut self, at: u64) -> u8 {
        <Bytes>::at(&mut self.bytes, at)
    }

    fn as_single_chunk(&mut self) -> Option<&[u8]> {
        let (_pos, chunk) = self.chunks.get()?;
        if chunk.as_ref().len() as u64 == self.slice.len() {
            self.chunk = Some(chunk);
            return Some(self.chunk.as_ref()?.as_ref());
        }

        None
    }

    fn copy_to(&mut self, at: u64, buf: &mut [u8]) -> usize {
        let mut pos_chunk = self.chunks.get();
        if pos_chunk.is_none() {
            self.chunks.prev();
        }

        while let Some((chunk_pos, _)) = pos_chunk.as_ref() {
            if *chunk_pos <= at {
                break;
            } else {
                pos_chunk = self.chunks.prev();
            }
        }

        let mut n = 0;
        while let Some((chunk_pos, chunk)) = pos_chunk {
            let chunk_bytes = chunk.as_ref();
            let start = if at > chunk_pos {
                (at - chunk_pos) as usize
            } else {
                0
            };

            if chunk_bytes.len() < start {
                pos_chunk = self.chunks.next();
                continue;
            }

            // log::info!(
            //     "start: {start}, min({}, {})",
            //     buf.len() - n,
            //     chunk_bytes.len() - start
            // );
            let end = start + std::cmp::min(buf.len() - n, chunk_bytes.len() - start);
            // log::info!(
            //     "start: {start}, end: {end}, min({}, {})",
            //     buf.len() - n,
            //     chunk_bytes.len() - start
            // );
            // log::info!("chk: {chunk_pos} len: {}, at: {at}, start: {start}, end: {end}", chunk_bytes.len());

            if end > start {
                let to_copy = &chunk_bytes[start..end];
                // log::info!("Copy: {to_copy:?}");
                let buf_piece = &mut buf[n..n + to_copy.len()];
                buf_piece.copy_from_slice(to_copy);
                n += to_copy.len();

                if n == buf.len() {
                    break;
                }
            }

            // println!("next");
            pos_chunk = self.chunks.next();
        }

        // log::info!("ret n: {n}");
        n
    }
}

#[derive(Debug)]
pub struct FileSource {
    size: u64,
    file: File,
    buf: Box<[u8]>,
    buf_pos: u64,
    buf_len: usize,
    pub stop: Arc<AtomicBool>,
}

impl FileSource {
    const SIZE: usize = 1024 * 256;

    pub fn new(path: &Path) -> io::Result<FileSource> {
        let size = path.metadata()?.len();
        let mut file = File::open(path)?;
        let mut buf = vec![0u8; Self::SIZE].into_boxed_slice();
        let n = Self::fill(&mut buf, &mut file, 0)?;

        Ok(FileSource {
            file,
            buf_pos: 0,
            buf,
            buf_len: n,
            size,
            stop: Arc::new(AtomicBool::new(false)),
        })
    }

    pub fn fill(buf: &mut [u8], file: &mut File, offset: u64) -> io::Result<usize> {
        file.seek(io::SeekFrom::Start(offset))?;

        let mut n = 0;
        while n < buf.len() {
            let read = file.read(buf)?;
            if read == 0 {
                break;
            }

            n += read;
        }

        Ok(n)
    }
}

impl ByteSource for FileSource {
    fn as_single_chunk(&mut self) -> Option<&[u8]> {
        if self.size == self.buf_len as u64 {
            return Some(&self.buf);
        }

        None
    }

    fn len(&self) -> u64 {
        self.size
    }

    fn stop(&self) -> bool {
        self.stop.load(Ordering::Acquire)
    }

    fn get(&mut self, at: u64) -> u8 {
        if at < self.buf_pos || (self.buf_pos + self.buf.len() as u64) <= at {
            let block = at / Self::SIZE as u64;
            let off = block * Self::SIZE as u64;
            match Self::fill(&mut self.buf, &mut self.file, off) {
                Ok(n) => {
                    self.buf_pos = off;
                    self.buf_len = n;
                }
                _ => {
                    self.stop.store(true, Ordering::Release);
                    return 0;
                }
            }
        }

        let relative = (at - self.buf_pos) as usize;
        self.buf[relative]
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use sanedit_buffer::PieceTree;

    #[test]
    fn slice_source() {
        let mut pt = PieceTree::new();
        let base = "hello world ".repeat(20);
        pt.insert(0, base.as_bytes());

        pt.insert(21, b"aaa");
        pt.insert(20, b"aaa");
        pt.insert(15, b"aaa");
        pt.insert(12, b"aaa");

        let slice = pt.slice(10..);
        let len = slice.len();
        let mut source = PTSliceSource::new(&slice);
        let mut buf = [0u8; 10];
        let mut n = 0;
        while n < len {
            let l = source.copy_to(n, &mut buf);
            // println!("{:?}", std::str::from_utf8(&buf[..l]).unwrap());
            n += l as u64;
        }

        assert!(n == len);
    }
}
