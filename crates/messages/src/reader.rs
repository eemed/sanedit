use std::io;

use serde::{Deserialize, Serialize};
use tokio_util::bytes::{Buf as _, BytesMut};
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
}

impl<R: io::Read, T> Iterator for Reader<R, T>
where
    for<'de> T: Deserialize<'de> + Serialize,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        BinCodec::<T>::decode_fill(&mut self.read, &mut self.buf).ok()?;

        match self.codec.decode(&mut self.buf) {
            Ok(Some(msg)) => Some(msg),
            Ok(None) => unreachable!("BinCodec::<T>::decode_fill not working as intended"),
            Err(_e) => {
                // Try to advance and retry
                self.buf.advance(1);
                self.next()
            }
        }
    }
}
