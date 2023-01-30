use crate::common::char::Char;

#[derive(Debug, Clone)]
pub(crate) enum Cell {
    Empty,
    Char {
        ch: Char,
        // style: Style,
    },
}

impl Cell {
    pub fn char(&self) -> Option<&Char> {
        match self {
            Cell::Empty => None,
            Cell::Char { ch } => Some(ch),
        }
    }
}

impl Default for Cell {
    fn default() -> Self {
        Cell::Empty
    }
}

impl From<Char> for Cell {
    fn from(ch: Char) -> Self {
        Cell::Char { ch }
    }
}
