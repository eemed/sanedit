use serde::{Deserialize, Serialize};

use super::{CursorStyle, Point};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Copy, Default)]
pub struct Cursor {
    pub style: CursorStyle,
    pub point: Point,
}
