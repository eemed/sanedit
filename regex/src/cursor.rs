pub trait CharCursor {
    fn at_start(&self) -> bool;
    fn at_end(&self) -> bool;
    fn next(&mut self) -> Option<char>;
    fn prev(&mut self) -> Option<char>;
}

pub(crate) struct StringCursor<'a> {
    pos: usize,
    string: &'a str,
}

impl<'a> StringCursor<'a> {
    pub fn new(string: &str) -> StringCursor {
        StringCursor { string, pos: 0 }
    }
}

impl<'a> CharCursor for StringCursor<'a> {
    fn at_start(&self) -> bool {
        self.pos == 0
    }

    fn at_end(&self) -> bool {
        self.pos == self.string.len()
    }

    fn next(&mut self) -> Option<char> {
        let part = &self.string[self.pos..];
        let ch = part.chars().next()?;
        self.pos += ch.len_utf8();
        Some(ch)
    }

    fn prev(&mut self) -> Option<char> {
        let part = &self.string[..self.pos];
        let ch = part.chars().rev().next()?;
        self.pos -= ch.len_utf8();
        Some(ch)
    }
}
