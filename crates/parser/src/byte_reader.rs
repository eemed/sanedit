use std::ops::Range;

pub trait ByteReader {
    /// Length of all the bytes in this reader
    fn len(&self) -> usize;

    /// Get a part of the bytes.
    /// Returns an iterator of byte blocks to support non contiguous inputs
    fn slice<'s>(&'s self, range: Range<usize>) -> Box<dyn Iterator<Item = &'s [u8]> + 's>;

    /// Wether to stop parsing and return an error
    fn stop(&self) -> bool;
}

impl<'a> ByteReader for &'a str {
    fn len(&self) -> usize {
        self.as_bytes().len()
    }

    fn stop(&self) -> bool {
        false
    }

    fn slice<'s>(&'s self, range: Range<usize>) -> Box<dyn Iterator<Item = &'s [u8]> + 's> {
        let bytes = &self.as_bytes()[range];
        let iter = std::iter::once(bytes);
        Box::new(iter)
    }
}
