use serde::{Deserialize, Serialize};

use super::{Color, CursorShape, Point};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Copy, Default)]
pub struct Cursor {
    pub bg: Option<Color>,
    pub fg: Option<Color>,
    pub shape: CursorShape,
    pub point: Point,
}
