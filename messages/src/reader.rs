use std::io;

use crate::ClientMessage;

/// Helper struct to read arbitrary amounts of bytes into memory from a reader.
/// It is different from a BufReader as it does not itself implement read, but
/// uses explicit consume call to pop bytes off the front and more call to
/// read more into the internal buffer.
pub struct Reader<R: io::Read> {
    read: R,
    buf: Vec<u8>,
}

impl<R: io::Read> Reader<R> {
    #[inline]
    pub fn new(read: R) -> Reader<R> {
        Reader {
            read,
            buf: Vec::new(),
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
    pub fn consume(&mut self, len: u64) {
        for _ in 0..len {
            self.buf.remove(0);
        }
    }

    #[inline]
    pub fn buffer(&self) -> &[u8] {
        &self.buf
    }
}

// impl<R: io::Read> Iterator for Reader<R> {
//     type Item = ClientMessage;

//     fn next(&mut self) -> Option<Self::Item> {
//         loop {
//             match ClientMessage::deserialize(self.buf.as_ref()) {
//                 Ok(msg) => {
//                     let size = msg.serialized_size().unwrap();
//                     // TODO optimize
//                     for _ in 0..size {
//                         self.buf.remove(0);
//                     }
//                     return Some(msg);
//                 }
//                 Err(e) => match e {
//                     crate::Error::Io(_) => {
//                         return None;
//                     }
//                     crate::Error::InvalidData => {
//                         // Move one byte to the right
//                         // TODO optimize
//                         self.buf.remove(0);
//                     }
//                     crate::Error::NeedMore => {
//                         self.read_more().ok()?;
//                     }
//                 },
//             }
//         }
//     }
// }
