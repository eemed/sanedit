use sanedit_buffer::{Bytes, PieceTree};
use sanedit_regex::Cursor;

#[derive(Debug)]
pub(crate) struct BufferCursor<'a> {
    len: usize,
    bytes: Bytes<'a>,
}

impl<'a> BufferCursor<'a> {
    pub fn new(slice: &'a PieceTree) -> BufferCursor<'a> {
        let len = slice.len();
        let bytes = slice.bytes();
        BufferCursor { bytes, len }
    }
}

impl<'a> Cursor for BufferCursor<'a> {
    fn at_start(&self) -> bool {
        self.bytes.pos() == 0
    }

    fn at_end(&self) -> bool {
        self.bytes.pos() == self.len
    }

    fn next(&mut self) -> Option<u8> {
        self.bytes.next()
    }

    fn prev(&mut self) -> Option<u8> {
        self.bytes.prev()
    }

    fn pos(&self) -> usize {
        self.bytes.pos()
    }
}
