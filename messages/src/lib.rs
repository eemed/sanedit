// An event is a message which informs various listeners about something which
// has happened.
// Commands trigger something which should happen (in the future).

mod key;
mod mouse;
mod reader;
mod redraw;

use std::io;

pub use key::{Key, KeyEvent, KeyMods};
pub use mouse::MouseEvent;
pub use reader::Reader;
pub use redraw::Redraw;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error")]
    Io(#[from] io::Error),

    #[error("Encountered invalid data")]
    InvalidData,

    #[error("Need more data")]
    NeedMore,
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
    pub fn serialize(&self) -> Result<Vec<u8>, Error> {
        match bincode::serialize(self) {
            Ok(bytes) => Ok(bytes),
            Err(e) => match *e {
                bincode::ErrorKind::Io(io) => Err(Error::Io(io)),
                _ => Err(Error::InvalidData),
            },
        }
    }

    pub fn serialized_size(&self) -> Result<u64, Error> {
        match bincode::serialized_size(self) {
            Ok(size) => Ok(size),
            Err(e) => match *e {
                bincode::ErrorKind::Io(io) => match io.kind() {
                    io::ErrorKind::UnexpectedEof => Err(Error::NeedMore),
                    _ => Err(Error::Io(io)),
                },
                _ => Err(Error::InvalidData),
            },
        }
    }

    pub fn deserialize(bytes: &[u8]) -> Result<ClientMessage, Error> {
        match bincode::deserialize::<ClientMessage>(bytes) {
            Ok(msg) => Ok(msg),
            Err(e) => match *e {
                bincode::ErrorKind::Io(io) => match io.kind() {
                    io::ErrorKind::UnexpectedEof => Err(Error::NeedMore),
                    _ => Err(Error::Io(io)),
                },
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
    pub fn serialize(&self) -> Result<Vec<u8>, Error> {
        match bincode::serialize(self) {
            Ok(bytes) => Ok(bytes),
            Err(e) => match *e {
                bincode::ErrorKind::Io(io) => Err(Error::Io(io)),
                _ => Err(Error::InvalidData),
            },
        }
    }
    pub fn serialized_size(&self) -> Result<u64, Error> {
        match bincode::serialized_size(self) {
            Ok(size) => Ok(size),
            Err(e) => match *e {
                bincode::ErrorKind::Io(io) => match io.kind() {
                    io::ErrorKind::UnexpectedEof => Err(Error::NeedMore),
                    _ => Err(Error::Io(io)),
                },
                _ => Err(Error::InvalidData),
            },
        }
    }

    pub fn deserialize(bytes: &[u8]) -> Result<Message, Error> {
        match bincode::deserialize::<Message>(bytes) {
            Ok(msg) => Ok(msg),
            Err(e) => match *e {
                bincode::ErrorKind::Io(io) => match io.kind() {
                    io::ErrorKind::UnexpectedEof => Err(Error::NeedMore),
                    _ => Err(Error::Io(io)),
                },
                _ => Err(Error::InvalidData),
            },
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn deserialize_hello() {
        let hello = ClientMessage::Hello;
        let bytes = hello.serialize().unwrap();
        let deserialized = ClientMessage::deserialize(&bytes).unwrap();
        assert_eq!(deserialized, hello);
    }

    #[test]
    fn invalid_bytes() {
        let bytes = [5, 0, 0, 0, 0];
        let err = ClientMessage::deserialize(&bytes).unwrap_err();
        assert_eq!("Encountered invalid data", &err.to_string());
    }

    #[test]
    fn not_enough_bytes() {
        let bytes = [0, 0, 0];
        let err = ClientMessage::deserialize(&bytes).unwrap_err();
        assert_eq!("Need more data", &err.to_string());
    }
}
