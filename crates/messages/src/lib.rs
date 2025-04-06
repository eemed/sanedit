// An event is a message which informs various listeners about something which
// has happened.
// Commands trigger something which should happen (in the future).

mod codec;
mod mouse;
mod reader;
mod writer;

pub mod key;
pub mod redraw;

use std::path::PathBuf;

pub use codec::BinCodec;
pub use mouse::{MouseButton, MouseEvent, MouseEventKind};
pub use reader::Reader;
pub use tokio_util::codec::{Decoder, Encoder};
pub use writer::{WriteError, Writer};

use key::KeyEvent;
use redraw::{Redraw, Size, Theme};
use serde::{Deserialize, Serialize};

/// Messages sent to the client
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub enum ClientMessage {
    Hello,
    Theme(Theme),
    Redraw(Redraw),
    Flush,
    Bye,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub enum Command {
    OpenFile(PathBuf),
}

/// Messages sent to the server
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub enum Message {
    Hello(Size),
    Command(Command),
    KeyEvent(KeyEvent),
    MouseEvent(MouseEvent),
    Resize(Size),
    Bye,
}
