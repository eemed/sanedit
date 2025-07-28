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

/// Trait to diff objects and apply them back into the struct
pub trait Diffable {
    type Diff;

    /// Calculate difference between self and other
    fn diff(&self, other: &Self) -> Option<Self::Diff>;

    /// Apply difference to self
    fn update(&mut self, diff: Self::Diff);
}

/// Component sent to the client. Components can be opened, updated and closed.
/// Updating is done through diffs to reduce data sent.
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub enum Component<F, D>
where
    F: Diffable<Diff = D>,
{
    Open(F),
    Update(D),
    Close,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub enum Redraw {
    Window(Component<window::Window, window::Difference>),
    Statusline(Component<statusline::Statusline, statusline::Difference>),
    Prompt(Component<prompt::Prompt, prompt::Difference>),
    Completion(Component<completion::Completion, completion::Difference>),
    Filetree(Component<items::Items, items::Difference>),
    Locations(Component<items::Items, items::Difference>),
    StatusMessage(StatusMessage),
    // LineNumbers(LineNumbers),
    Popup(PopupComponent),
}

impl From<Redraw> for ClientMessage {
    fn from(value: Redraw) -> Self {
        ClientMessage::Redraw(value)
    }
}
