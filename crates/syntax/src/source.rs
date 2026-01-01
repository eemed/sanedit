use std::{
    cmp::min,
    io::{self, Read, Seek, SeekFrom},
    ops::Range,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use sanedit_buffer::{utf8::decode_utf8, Chunks, PieceTreeSlice};

const BUF_SIZE: usize = 64 * 1024;

pub trait Source {
    fn buffer(&self) -> (u64, &[u8]);
    fn refill_buffer(&mut self, pos: u64) -> io::Result<bool>;
    fn refill_buffer_rev(&mut self, pos: u64) -> io::Result<bool>;
    fn len(&self) -> u64;
    fn slice(&mut self, range: Range<u64>) -> Option<&[u8]>;
    fn get(&mut self, at: u64) -> Option<u8>;

    fn stop(&self) -> bool {
        false
    }

    fn stop_flag(&mut self, _flag: Arc<AtomicBool>) {}

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn is_in_memory(&self) -> bool {
        let (pos, buf) = self.buffer();
        pos == 0 && buf.len() as u64 == self.len()
    }

    fn char_between(&mut self, at: u64, start: char, end: char) -> Option<u64> {
        let max = min(4, self.len() - at);
        let bytes = self.slice(at..at + max)?;
        let (ch, size) = decode_utf8(bytes);
        let ch = ch?;

        if start <= ch && ch <= end {
            Some(size as u64)
        } else {
            None
        }
    }

    fn matches_self(&mut self, at: u64, at2: u64, len: u64) -> bool {
        let one = self.slice(at..at + len).map(|bytes| bytes.to_vec());
        let two = self.slice(at2..at2 + len);
        one.is_some() && one.as_deref() == two
    }
}

impl<const N: usize> Source for &[u8; N] {
    fn buffer(&self) -> (u64, &[u8]) {
        (0, *self)
    }

    fn refill_buffer(&mut self, _pos: u64) -> io::Result<bool> {
        Ok(false)
    }

    fn refill_buffer_rev(&mut self, _pos: u64) -> io::Result<bool> {
        Ok(false)
    }

    fn len(&self) -> u64 {
        N as u64
    }

    fn slice(&mut self, range: Range<u64>) -> Option<&[u8]> {
        let start = range.start as usize;
        let end = range.end as usize;
        if N < end {
            None
        } else {
            Some(&self[start..end])
        }
    }

    fn get(&mut self, at: u64) -> Option<u8> {
        let at = at as usize;
        if N <= at {
            None
        } else {
            Some(self[at])
        }
    }
}

impl Source for &[u8] {
    fn buffer(&self) -> (u64, &[u8]) {
        (0, self)
    }

    fn refill_buffer(&mut self, _pos: u64) -> io::Result<bool> {
        Ok(false)
    }

    fn refill_buffer_rev(&mut self, _pos: u64) -> io::Result<bool> {
        Ok(false)
    }

    fn len(&self) -> u64 {
        <[u8]>::len(self) as u64
    }

    fn slice(&mut self, range: Range<u64>) -> Option<&[u8]> {
        let start = range.start as usize;
        let end = range.end as usize;
        if <[u8]>::len(self) < end {
            None
        } else {
            Some(&self[start..end])
        }
    }

    fn get(&mut self, at: u64) -> Option<u8> {
        let at = at as usize;
        if <[u8]>::len(self) <= at {
            None
        } else {
            Some(self[at])
        }
    }
}

impl Source for &str {
    fn buffer(&self) -> (u64, &[u8]) {
        (0, self.as_bytes())
    }

    fn refill_buffer(&mut self, _pos: u64) -> io::Result<bool> {
        Ok(false)
    }

    fn refill_buffer_rev(&mut self, _pos: u64) -> io::Result<bool> {
        Ok(false)
    }

    fn len(&self) -> u64 {
        <str>::len(self) as u64
    }

    fn slice(&mut self, range: Range<u64>) -> Option<&[u8]> {
        let start = range.start as usize;
        let end = range.end as usize;
        if <str>::len(self) < end {
            None
        } else {
            Some(&self.as_bytes()[start..end])
        }
    }

    fn get(&mut self, at: u64) -> Option<u8> {
        let at = at as usize;
        if <str>::len(self) <= at {
            None
        } else {
            Some(self.as_bytes()[at])
        }
    }
}

#[derive(Debug)]
pub struct BufferedSource<R: Read + Seek> {
    reader: R,
    buf: Box<[u8]>,
    buf_len: usize,
    pos: u64,
    len: u64,
    stop: Arc<AtomicBool>,
}

impl<R: Read + Seek> BufferedSource<R> {
    pub fn new(mut reader: R) -> io::Result<BufferedSource<R>> {
        let len = reader.seek(SeekFrom::End(0))?;
        let mut source = BufferedSource {
            reader,
            buf: vec![0u8; BUF_SIZE].into_boxed_slice(),
            buf_len: 0,
            pos: 0,
            len,
            stop: Arc::new(AtomicBool::new(false)),
        };
        let _ = source.refill_buffer(0);

        Ok(source)
    }

    pub fn with_stop(reader: R, stop: Arc<AtomicBool>) -> io::Result<BufferedSource<R>> {
        let mut me = Self::new(reader)?;
        me.stop_flag(stop);
        Ok(me)
    }
}

impl<R: Read + Seek> Source for BufferedSource<R> {
    fn buffer(&self) -> (u64, &[u8]) {
        let pos = self.pos;
        let buf = &self.buf[..self.buf_len];
        (pos, buf)
    }

    fn refill_buffer(&mut self, pos: u64) -> io::Result<bool> {
        debug_assert!(
            pos <= self.len,
            "Requesting pos {pos} from source of length {}",
            self.len
        );
        if self.pos == pos && self.buf_len != 0 {
            return Ok(false);
        };

        let mut n = 0;

        if self.pos < pos && pos < self.pos + (self.buf_len as u64) {
            let start = (pos - self.pos) as usize;
            let end = self.buf_len;
            self.buf.copy_within(start..end, 0);
            n = end - start;
        }

        self.reader.seek(SeekFrom::Start(pos + n as u64))?;

        while n < self.buf.len() {
            let amount = self.reader.read(&mut self.buf[n..])?;
            if amount == 0 {
                break;
            }
            n += amount;
        }

        if n == 0 {
            return Ok(false);
        }

        self.pos = pos;
        self.buf_len = n;

        Ok(true)
    }

    fn refill_buffer_rev(&mut self, pos: u64) -> io::Result<bool> {
        let start = pos.saturating_sub(self.buf.len() as u64);
        self.refill_buffer(start)
    }

    fn len(&self) -> u64 {
        self.len
    }

    fn slice(&mut self, range: Range<u64>) -> Option<&[u8]> {
        // Assume mostly forward
        let in_range = self.pos <= range.start && range.end <= (self.pos + self.buf_len as u64);
        if !in_range && !self.refill_buffer(range.start).ok()? {
            return None;
        }

        let relative_start = (range.start - self.pos) as usize;
        let relative_end = (range.end - self.pos) as usize;

        Some(&self.buf[relative_start..relative_end])
    }

    fn get(&mut self, at: u64) -> Option<u8> {
        let in_range = self.pos <= at && at < self.pos + self.buf_len as u64;
        if !in_range && !self.refill_buffer(at).ok()? {
            return None;
        }

        let relative = (at - self.pos) as usize;
        Some(self.buf[relative])
    }

    fn stop(&self) -> bool {
        self.stop.load(Ordering::Acquire)
    }

    fn stop_flag(&mut self, flag: Arc<AtomicBool>) {
        self.stop = flag;
    }
}

#[derive(Debug)]
struct SliceChunks<'a> {
    chunks: Chunks<'a>,
    pos: u64,
}

impl<'a> SliceChunks<'a> {
    pub fn new(slice: &PieceTreeSlice) -> SliceChunks<'_> {
        SliceChunks {
            chunks: slice.chunks(),
            pos: 0,
        }
    }

    fn prepare_read(&mut self) -> Option<()> {
        if self.chunks.get().is_none() {
            self.chunks.prev();
        }
        let (mut chunk_pos, mut chunk) = self.chunks.get()?;

        while self.pos < chunk_pos {
            (chunk_pos, chunk) = self.chunks.prev()?;
        }

        while chunk_pos + (chunk.as_ref().len() as u64) <= self.pos {
            (chunk_pos, chunk) = self.chunks.next()?;
        }

        Some(())
    }
}

impl<'a> Read for SliceChunks<'a> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.prepare_read().is_none() {
            return Ok(0);
        }

        let Some((chunk_pos, chunk)) = self.chunks.get() else {
            return Ok(0);
        };

        let in_range =
            chunk_pos <= self.pos && self.pos < chunk_pos + (chunk.as_ref().len() as u64);

        if !in_range {
            return Ok(0);
        }

        let relative = (self.pos - chunk_pos) as usize;
        let mut bytes = &chunk.as_ref()[relative..];
        let n = bytes.read(buf)?;
        self.pos += n as u64;

        Ok(n)
    }
}

impl<'a> Seek for SliceChunks<'a> {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let end = self.chunks.slice().len();

        match pos {
            SeekFrom::Start(n) => {
                debug_assert!(n <= end);
                self.pos = n;
            }
            SeekFrom::End(n) => {
                debug_assert!(n >= 0);
                let abs = n.unsigned_abs();

                debug_assert!(abs <= end);
                self.pos = end - abs;
            }
            SeekFrom::Current(n) => {
                let abs = n.unsigned_abs();
                if n.is_negative() {
                    debug_assert!(abs <= self.pos);
                    self.pos -= abs;
                } else {
                    debug_assert!(abs + self.pos <= end);
                    self.pos += abs;
                }
            }
        }

        Ok(self.pos)
    }
}

#[derive(Debug)]
pub struct PieceTreeSliceSource<'a>(BufferedSource<SliceChunks<'a>>);

impl<'a> PieceTreeSliceSource<'a> {
    pub fn new(slice: &PieceTreeSlice) -> io::Result<PieceTreeSliceSource<'_>> {
        let inner = BufferedSource::new(SliceChunks::new(slice))?;
        Ok(PieceTreeSliceSource(inner))
    }

    pub fn with_stop(
        slice: &PieceTreeSlice,
        stop: Arc<AtomicBool>,
    ) -> io::Result<PieceTreeSliceSource<'_>> {
        let mut me = Self::new(slice)?;
        me.stop_flag(stop);
        Ok(me)
    }
}

impl<'a> Source for PieceTreeSliceSource<'a> {
    fn buffer(&self) -> (u64, &[u8]) {
        self.0.buffer()
    }

    fn refill_buffer(&mut self, pos: u64) -> io::Result<bool> {
        self.0.refill_buffer(pos)
    }

    fn refill_buffer_rev(&mut self, pos: u64) -> io::Result<bool> {
        self.0.refill_buffer_rev(pos)
    }

    fn len(&self) -> u64 {
        self.0.len()
    }

    fn slice(&mut self, range: Range<u64>) -> Option<&[u8]> {
        self.0.slice(range)
    }

    fn get(&mut self, at: u64) -> Option<u8> {
        self.0.get(at)
    }
}
