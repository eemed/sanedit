mod cell;
mod color;
mod completion;
mod cursor;
mod cursor_shape;
mod filetree;
mod line_numbers;
mod point;
mod prompt;
mod size;
mod status_message;
mod statusline;
mod style;
mod text_style;
mod theme;
mod window;

pub use cell::*;
pub use color::*;
pub use completion::*;
pub use cursor::*;
pub use cursor_shape::*;
pub use filetree::*;
pub use line_numbers::*;
pub use point::*;
pub use prompt::*;
pub use size::*;
pub use status_message::*;
pub use statusline::*;
pub use style::*;
pub use text_style::*;
pub use theme::*;
pub use window::*;

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
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub enum Component<F, D>
where
    F: Diffable<Diff = D>,
{
    Open(F),
    Update(D),
    Close,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub enum Redraw {
    Window(Component<Window, window::Difference>),
    Statusline(Component<Statusline, statusline::Difference>),
    Prompt(Component<Prompt, prompt::Difference>),
    Completion(Component<Completion, completion::Difference>),
    Filetree(Component<Filetree, filetree::Difference>),
    StatusMessage(StatusMessage),
    LineNumbers(LineNumbers),
}

impl From<Redraw> for ClientMessage {
    fn from(value: Redraw) -> Self {
        ClientMessage::Redraw(value)
    }
}
