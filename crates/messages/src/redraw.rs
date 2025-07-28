mod cell;
mod color;
mod cursor;
mod cursor_shape;
// mod line_numbers;
mod point;
mod popup;
mod size;
mod status_message;
mod style;
mod theme;

pub mod choice;
pub mod completion;
pub mod items;
pub mod prompt;
pub mod statusline;
pub mod text_style;
pub mod window;

pub use cell::*;
pub use color::*;
pub use cursor::*;
pub use cursor_shape::*;
pub use point::*;
pub use popup::*;
pub use size::*;
pub use status_message::*;
pub use style::*;
pub use theme::*;

pub use sanedit_core::Severity;

use serde::{Deserialize, Serialize};

use crate::ClientMessage;

/// Component sent to the client. Components can be opened, updated and closed.
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub enum Component<F> {
    Open(F),
    Update(F),
    Close,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub enum Redraw {
    Window(Component<window::Window>),
    Statusline(Component<statusline::Statusline>),
    Prompt(Component<prompt::Prompt>),
    Completion(Component<completion::Completion>),
    Filetree(Component<items::Items>),
    Locations(Component<items::Items>),
    StatusMessage(StatusMessage),
    // LineNumbers(LineNumbers),
    Popup(PopupComponent),
}

impl From<Redraw> for ClientMessage {
    fn from(value: Redraw) -> Self {
        ClientMessage::Redraw(value)
    }
}
