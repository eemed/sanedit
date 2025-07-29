use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum CursorShape {
    Block(bool),
    Underline(bool),
    Line(bool),
}

impl CursorShape {
    pub fn blink(&self) -> bool {
        match self {
            CursorShape::Block(b) => *b,
            CursorShape::Underline(b) => *b,
            CursorShape::Line(b) => *b,
        }
    }
}

impl Default for CursorShape {
    fn default() -> Self {
        CursorShape::Block(false)
    }
}
