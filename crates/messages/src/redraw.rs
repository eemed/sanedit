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
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Hash)]
pub enum Component<F> {
    Update(F),
    Close,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Hash)]
pub enum Kind {
    Prompt,
    Completion,
    Filetree,
    Locations,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Hash)]
pub enum Redraw {
    WindowCursor(Option<Cursor>),
    Window(Component<window::Window>),
    Statusline(Component<statusline::Statusline>),
    /// Updates just the selection
    Selection(Kind, Option<usize>),
    Prompt(Component<prompt::Prompt>),
    Completion(Component<completion::Completion>),
    Filetree(Component<items::Items>),
    Locations(Component<items::Items>),
    StatusMessage(StatusMessage),
    Popup(PopupComponent),
}

impl From<Redraw> for ClientMessage {
    fn from(value: Redraw) -> Self {
        ClientMessage::Redraw(value)
    }
}
