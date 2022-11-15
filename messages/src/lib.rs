// An event is a message which informs various listeners about something which
// has happened.
// Commands trigger something which should happen (in the future).

mod key;
mod mouse;

pub use key::KeyEvent;
pub use mouse::MouseEvent;
use serde::{Deserialize, Serialize};

/// Messages sent to the client
#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub enum ClientMessage {
    Hello,
    Redraw,
    Flush,
    Bye,
}

impl ClientMessage {
    pub fn to_bytes(&self) -> Result<Vec<u8>, Box<bincode::ErrorKind>> {
        bincode::serialize(self)
    }
}

/// Messages sent to the server
#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub enum Message {
    Hello,
    KeyEvent(KeyEvent),
    MouseEvent(MouseEvent),
    Resize,
    Bye,
}


