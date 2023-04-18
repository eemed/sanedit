use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Copy)]
pub enum CursorStyle {
    Block(bool),
    Underline(bool),
    Line(bool),
}

impl CursorStyle {
    pub fn blink(&self) -> bool {
        match self {
            CursorStyle::Block(b) => *b,
            CursorStyle::Underline(b) => *b,
            CursorStyle::Line(b) => *b,
        }
    }
}

impl Default for CursorStyle {
    fn default() -> Self {
        CursorStyle::Block(false)
    }
}
