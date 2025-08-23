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
    Drag(MouseButton),
}

#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq, Clone)]
pub struct MouseEvent {
    pub kind: MouseEventKind,
    pub mods: KeyMods,
    pub point: Point,
    pub element: Element,
}

impl From<MouseEvent> for Message {
    fn from(event: MouseEvent) -> Self {
        Message::MouseEvent(event)
    }
}

#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq, Clone)]
pub enum Element {
    Filetree,
    Locations,
    Window,
}
