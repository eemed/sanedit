use serde::{Deserialize, Serialize};

use crate::Message;

#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq, Clone)]
pub enum MouseEvent {
    ScrollDown,
    ScrollUp,
}

impl From<MouseEvent> for Message {
    fn from(event: MouseEvent) -> Self {
        Message::MouseEvent(event)
    }
}
