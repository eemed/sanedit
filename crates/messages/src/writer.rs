use std::io;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio_util::bytes::BytesMut;
use tokio_util::codec::Encoder;

use crate::BinCodec;

pub struct Writer<W: io::Write, T> {
    codec: BinCodec<T>,
    buf: BytesMut,
    write: W,
}

impl<W: io::Write, T> Writer<W, T>
where
    for<'de> T: Serialize,
{
    #[inline]
    pub fn new(write: W) -> Writer<W, T> {
        Writer {
            write,
            buf: BytesMut::new(),
            codec: BinCodec::new(),
        }
    }

    pub fn write(&mut self, msg: T) -> Result<(), WriteError> {
        self.codec.encode(msg, &mut self.buf)?;
        self.write.write_all(&self.buf)?;
        self.buf.clear();
        Ok(())
    }

    pub fn write_ref(&mut self, msg: &T) -> Result<(), WriteError> {
        self.codec.encode(msg, &mut self.buf)?;
        self.write.write_all(&self.buf)?;
        self.buf.clear();
        Ok(())
    }
}

#[derive(Error, Debug)]
pub enum WriteError {
    #[error("Failed to encode message")]
    Encoding(#[from] Box<bincode::ErrorKind>),

    #[error("Write error")]
    IoError(#[from] io::Error),
}
