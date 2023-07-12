mod cell;
mod color;
mod completion;
mod cursor;
mod cursor_shape;
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

/// Trait to diff objects and apply them back into the object
pub trait Diffable {
    type Diff;

    /// Calculate difference between self and other
    fn diff(&self, other: &Self) -> Option<Self::Diff>;

    /// Apply difference to self
    fn update(&mut self, diff: Self::Diff);
}

pub enum Component<D, F: Diffable<Diff = D>> {
    Open(F),
    Update(D),
    Close,
}

impl<D, F: Diffable<Diff = D>> From<F> for Component<D, F> {
    fn from(value: F) -> Self {
        Self::Open(value)
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub enum Redraw {
    // Statusline and window cannot currently be closed so they do not have a
    // close redraw event
    /// First draw for window and statusline
    Init(Window, Statusline),
    /// Window updated
    WindowUpdate(WindowDiff),

    /// Statusline updated
    StatuslineUpdate(StatuslineDiff),

    Prompt(Prompt),
    PromptUpdate(PromptDiff),
    ClosePrompt,

    Completion(Completion),
    CompletionUpdate(completion::Difference),
    CloseCompletion,

    // Completion2(Component<Completion, completion::Difference>),
    // Prompt2(Component<Prompt, PromptDiff>),
    // Window2(Component<Window, WindowDiff>),
    // Statusline2(Component<Statusline, StatuslineDiff>),
    /// Status messages
    StatusMessage(StatusMessage),
}
