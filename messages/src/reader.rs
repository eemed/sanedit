use std::io;

use bytes::{Buf, BytesMut};
use serde::{Deserialize, Serialize};
use tokio_util::codec::Decoder;

use crate::BinCodec;

/// Helper struct to read arbitrary amounts of bytes into memory from a reader.
/// It is different from a BufReader as it does not itself implement read, but
/// uses explicit consume call to pop bytes off the front and more call to
/// read more into the internal buffer.
pub struct Reader<R: io::Read, T> {
    read: R,
    buf: BytesMut,
    codec: BinCodec<T>,
}

impl<R: io::Read, T> Reader<R, T>
where
    for<'de> T: Deserialize<'de> + Serialize,
{
    #[inline]
    pub fn new(read: R) -> Reader<R, T> {
        Reader {
            read,
            buf: BytesMut::new(),
            codec: BinCodec::new(),
        }
    }

    /// Read more bytes to the internal buffer from the provided reader.
    #[inline]
    fn read_more_to_buf(&mut self) -> io::Result<usize> {
        let mut read_buf = [0u8; 1024 * 8];
        let size = self.read.read(&mut read_buf)?;
        self.buf.extend(&read_buf[..size]);
        Ok(size)
    }
}

impl<R: io::Read, T> Iterator for Reader<R, T>
where
    for<'de> T: Deserialize<'de> + Serialize,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.codec.decode(&mut self.buf) {
                Ok(Some(msg)) => {
                    return Some(msg);
                }
                Ok(None) => match self.read_more_to_buf() {
                    Ok(0) | Err(_) => return None,
                    Ok(n) => log::info!("read {n} bytes, {}", self.buf.len()),
                },
                Err(_e) => {
                    self.buf.advance(1);
                }
            }
        }
    }
}
