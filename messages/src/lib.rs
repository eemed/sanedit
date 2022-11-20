// An event is a message which informs various listeners about something which
// has happened.
// Commands trigger something which should happen (in the future).

mod key;
mod mouse;
mod redraw;

use std::io;

pub use key::{Key, KeyEvent, KeyMods};
pub use mouse::MouseEvent;
pub use redraw::Redraw;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error")]
    Io(#[from] io::Error),

    #[error("Encountered invalid data")]
    InvalidData,
}

/// Messages sent to the client
#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub enum ClientMessage {
    Hello,
    Redraw(Redraw),
    Flush,
    Bye,
}

impl ClientMessage {
    pub fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        match bincode::serialize(self) {
            Ok(bytes) => Ok(bytes),
            Err(e) => match *e {
                bincode::ErrorKind::Io(io) => Err(Error::Io(io)),
                _ => Err(Error::InvalidData),
            },
        }
    }

    pub fn from_reader<R: io::Read>(reader: R) -> Result<ClientMessage, Error> {
        match bincode::deserialize_from::<_, ClientMessage>(reader) {
            Ok(msg) => Ok(msg),
            Err(e) => match *e {
                bincode::ErrorKind::Io(io) => Err(Error::Io(io)),
                _ => Err(Error::InvalidData),
            },
        }
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

impl Message {
    pub fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        match bincode::serialize(self) {
            Ok(bytes) => Ok(bytes),
            Err(e) => match *e {
                bincode::ErrorKind::Io(io) => Err(Error::Io(io)),
                _ => Err(Error::InvalidData),
            },
        }
    }

    pub fn from_reader<R: io::Read>(reader: R) -> Result<Message, Error> {
        match bincode::deserialize_from::<_, Message>(reader) {
            Ok(msg) => Ok(msg),
            Err(e) => match *e {
                bincode::ErrorKind::Io(io) => Err(Error::Io(io)),
                _ => Err(Error::InvalidData),
            },
        }
    }
}
