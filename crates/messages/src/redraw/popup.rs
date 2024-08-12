use serde::{Deserialize, Serialize};

use super::{Point, Redraw, Severity};

/// A read only window that pops up at a position.
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct Popup {
    pub severity: Severity,
    pub point: Point,
    pub lines: Vec<String>,
}

impl From<Popup> for Redraw {
    fn from(msg: Popup) -> Self {
        Redraw::Popup(msg)
    }
}
