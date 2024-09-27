use std::{
    io::{self, Write as _},
    marker::PhantomData,
};

use bincode::Options;
use serde::{Deserialize, Serialize};
use tokio_util::{
    bytes::{Buf as _, BufMut as _, BytesMut},
    codec::{Decoder, Encoder},
};

const U32_BYTES: usize = 4;

pub struct BinCodec<T> {
    phantom: PhantomData<T>,
}

impl<T> BinCodec<T>
where
    for<'de> T: Deserialize<'de> + Serialize,
{
    pub fn new() -> BinCodec<T> {
        BinCodec {
            phantom: PhantomData,
        }
    }

    fn decode_fill_size<R: io::Read>(
        read: &mut R,
        src: &mut BytesMut,
        size: usize,
    ) -> io::Result<usize> {
        if src.len() >= size {
            return Ok(0);
        }

        let mut total = 0;

        if src.capacity() < size {
            src.reserve(size - src.capacity());
        }

        let mut rest = src.split_off(src.len());
        let restlen = rest.capacity();
        // SAFETY: Never reading unwritten bytes
        // and capacity is already reserved for us
        unsafe { rest.set_len(restlen) }

        while src.len() < size {
            let n = read.read(&mut rest[..])?;
            total += n;
            let good = rest.split_to(n);
            src.unsplit(good);
        }

        Ok(total)
    }

    /// Read enough bytes from read to src to be able to decode an item
    pub fn decode_fill<R: io::Read>(read: &mut R, src: &mut BytesMut) -> io::Result<usize> {
        let first = Self::decode_fill_size(read, src, U32_BYTES)?;
        let size = u32::from_be_bytes(src[..U32_BYTES].try_into().unwrap()) as usize;
        let second = Self::decode_fill_size(read, src, size + U32_BYTES)?;

        Ok(first + second)
    }
}

impl<T> Default for BinCodec<T>
where
    for<'de> T: Deserialize<'de> + Serialize,
{
    fn default() -> Self {
        BinCodec::new()
    }
}

impl<T> Decoder for BinCodec<T>
where
    for<'de> T: Deserialize<'de> + Serialize,
{
    type Item = T;

    type Error = bincode::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.remaining() < U32_BYTES {
            return Ok(None);
        }

        let size = u32::from_be_bytes(src[..U32_BYTES].try_into().unwrap()) as usize;
        if src.remaining() < U32_BYTES + size {
            return Ok(None);
        }

        src.advance(U32_BYTES);

        let item = bincode::options()
            .with_big_endian()
            .with_fixint_encoding()
            .allow_trailing_bytes()
            .with_limit(u32::MAX as u64)
            .deserialize::<T>(src)?;

        src.advance(size);
        Ok(Some(item))
    }
}

impl<T> Encoder<T> for BinCodec<T>
where
    for<'de> T: Deserialize<'de> + Serialize,
{
    type Error = bincode::Error;

    fn encode(&mut self, item: T, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let bcode = bincode::options()
            .with_big_endian()
            .with_fixint_encoding()
            .with_limit(u32::MAX as u64);

        let size = bcode.serialized_size(&item)? as u32;
        let total = U32_BYTES + size as usize;
        let available = dst.capacity() - dst.len();
        let missing = available.saturating_sub(total);
        if missing != 0 {
            dst.reserve(missing);
        }

        let mut writer = dst.writer();

        let bytes = size.to_be_bytes();
        writer.write(&bytes)?;
        bcode.serialize_into(writer, &item)
    }
}
