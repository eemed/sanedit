use crate::editor::common::char::Char;

#[derive(Debug, Default, Clone)]
pub(crate) struct Cell {
    ch: Char,
    // style: Style,
}

impl From<Char> for Cell {
    fn from(ch: Char) -> Self {
        Cell { ch }
    }
}
