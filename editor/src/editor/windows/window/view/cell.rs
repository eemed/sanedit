use crate::editor::common::char::Char;

#[derive(Debug, Default, Clone)]
pub(crate) struct Cell {
    ch: Char,
    // style: Style,
}

impl Cell {
    pub fn char(&self) -> &Char {
        &self.ch
    }
}

impl From<Char> for Cell {
    fn from(ch: Char) -> Self {
        Cell { ch }
    }
}
