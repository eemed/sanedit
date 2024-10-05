use sanedit_core::Severity;
use serde::{Deserialize, Serialize};

use super::{Point, Redraw};

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub enum PopupComponent {
    Open(Popup),
    Close,
}

/// A read only window that pops up at a position.
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct Popup {
    pub point: Point,
    pub messages: Vec<PopupMessage>,
    // Just for UI
    pub line_offset: usize,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct PopupMessage {
    pub severity: Option<Severity>,
    pub text: String,
}

impl From<Popup> for Redraw {
    fn from(msg: Popup) -> Self {
        Redraw::Popup(PopupComponent::Open(msg))
    }
}
