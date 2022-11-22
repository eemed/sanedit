use std::{io, marker::PhantomData};

use bytes::{Buf, BufMut, BytesMut};
use serde::{Deserialize, Serialize};
use tokio_util::codec::{Decoder, Encoder};

/// Helper struct to read arbitrary amounts of bytes into memory from a reader.
/// It is different from a BufReader as it does not itself implement read, but
/// uses explicit consume call to pop bytes off the front and more call to
/// read more into the internal buffer.
pub struct Reader<R: io::Read> {
    read: R,
    buf: BytesMut,
}

impl<R: io::Read> Reader<R> {
    #[inline]
    pub fn new(read: R) -> Reader<R> {
        Reader {
            read,
            buf: BytesMut::new(),
        }
    }

    /// Read more bytes to the internal buffer from the provided reader.
    #[inline]
    pub fn more(&mut self) -> io::Result<usize> {
        let mut read_buf = [0u8; 1024 * 8];
        let size = self.read.read(&mut read_buf)?;
        self.buf.extend(&read_buf[..size]);
        Ok(size)
    }

    /// Consumes the first len bytes from the internal buffer
    #[inline]
    pub fn advance(&mut self, len: usize) {
        self.buf.advance(len);
    }

    #[inline]
    pub fn buffer(&mut self) -> &mut BytesMut {
        &mut self.buf
    }
}

pub struct BinCodec<T> {
    phantom: PhantomData<T>,
}

impl<T> BinCodec<T>
where
    for<'de> T: Deserialize<'de> + Serialize,
{
    pub fn new() -> BinCodec<T> {
        BinCodec {
            phantom: PhantomData::default(),
        }
    }
}

impl<T> Decoder for BinCodec<T>
where
    for<'de> T: Deserialize<'de> + Serialize,
{
    type Item = T;

    type Error = bincode::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        match bincode::deserialize::<T>(&src) {
            Ok(item) => {
                let size = bincode::serialized_size(&item)? as usize;
                src.advance(size);
                Ok(Some(item))
            }
            Err(e) => {
                // TODO advance 1 on error bytes
                if let bincode::ErrorKind::Io(io) = e.as_ref() {
                    if let io::ErrorKind::UnexpectedEof = io.kind() {
                        return Ok(None);
                    }
                }

                Err(e)
            }
        }
    }
}

impl<T> Encoder<T> for BinCodec<T>
where
    for<'de> T: Deserialize<'de> + Serialize,
{
    type Error = bincode::Error;

    fn encode(&mut self, item: T, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let size = bincode::serialized_size(&item)? as usize;
        dst.reserve(size);
        bincode::serialize_into(dst.writer(), &item)
    }
}
