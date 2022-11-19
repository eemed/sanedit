use std::io;

/// Struct to read bytes and transform them back into messages.
#[derive(Debug)]
pub struct MessageReader<R: io::Read> {
    buf: Vec<u8>,
    reader: R,
}

impl<R: io::Read> MessageReader<R> {
    pub fn next<'a, T>() -> io::Result<T>
    where
        T: serde::de::Deserialize<'a>,
    {
        todo!()
    }
}
