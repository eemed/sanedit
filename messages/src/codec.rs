use std::{io, marker::PhantomData};

use bytes::{Buf, BufMut, BytesMut};
use serde::{Deserialize, Serialize};
use tokio_util::codec::{Decoder, Encoder};

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
