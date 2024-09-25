use serde::{Deserialize, Serialize};

use crate::{key::KeyMods, redraw::Point, Message};

#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq, Clone)]
pub enum MouseButton {
    Right,
    Middle,
    Left,
}

#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq, Clone)]
pub enum MouseEventKind {
    ScrollDown,
    ScrollUp,
    ButtonDown(MouseButton),
    ButtonUp(MouseButton),
}

#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq, Clone)]
pub struct MouseEvent {
    pub kind: MouseEventKind,
    pub mods: KeyMods,
    pub point: Point,
}

impl From<MouseEvent> for Message {
    fn from(event: MouseEvent) -> Self {
        Message::MouseEvent(event)
    }
}
