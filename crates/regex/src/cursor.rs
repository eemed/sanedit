pub trait Cursor {
    fn at_start(&self) -> bool;
    fn at_end(&self) -> bool;
    fn next(&mut self) -> Option<u8>;
    fn prev(&mut self) -> Option<u8>;
    fn pos(&self) -> usize;
}

pub struct StringCursor<'a> {
    pos: usize,
    string: &'a str,
}

impl<'a> StringCursor<'a> {
    pub fn new(string: &str) -> StringCursor {
        StringCursor { string, pos: 0 }
    }
}

impl<'a> Cursor for StringCursor<'a> {
    fn at_start(&self) -> bool {
        self.pos == 0
    }

    fn at_end(&self) -> bool {
        self.pos == self.string.len()
    }

    fn next(&mut self) -> Option<u8> {
        let bytes = self.string.as_bytes();
        if self.pos >= bytes.len() {
            return None;
        }

        let byte = bytes[self.pos];
        self.pos += 1;
        Some(byte)
    }

    fn prev(&mut self) -> Option<u8> {
        let bytes = self.string.as_bytes();
        self.pos = self.pos.saturating_sub(1);
        let byte = bytes[self.pos];
        Some(byte)
    }

    fn pos(&self) -> usize {
        self.pos
    }
}

impl<'a> From<&'a str> for StringCursor<'a> {
    fn from(s: &'a str) -> Self {
        Self::new(s)
    }
}
