use sanedit_core::Severity;
use serde::{Deserialize, Serialize};

use super::{Cell, Point, Redraw};

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Hash)]
pub enum PopupComponent {
    Open(Popup),
    Close,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Hash)]
pub enum PopupKind {
    SignatureHelp,
    Hover,
    Diagnostic,
}

/// A read only window that pops up at a position.
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Hash)]
pub struct Popup {
    pub point: Point,
    pub messages: Vec<PopupMessage>,
    // Just for UI
    pub line_offset: usize,
    pub kind: PopupKind,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Hash)]
pub struct PopupMessage {
    pub severity: Option<Severity>,
    pub text: PopupMessageText,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Hash)]
pub enum PopupMessageText {
    Formatted(Vec<Vec<Cell>>),
    Plain(String),
}

impl From<Popup> for Redraw {
    fn from(msg: Popup) -> Self {
        Redraw::Popup(PopupComponent::Open(msg))
    }
}
