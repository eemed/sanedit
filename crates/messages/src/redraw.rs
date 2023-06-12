mod cell;
mod color;
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

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub enum Redraw {
    /// First draw for window and statusline
    Init(Window, Statusline),
    /// Window updated
    WindowUpdate(WindowDiff),

    /// Statusline updated
    StatuslineUpdate(StatuslineDiff),

    // Statusline and window cannot currently be closed so they do not have a
    // close redraw event
    /// Prompt opened
    Prompt(Prompt),
    /// Prompt updated
    PromptUpdate(PromptDiff),
    /// Prompt closed
    ClosePrompt,

    /// Status messages
    StatusMessage(StatusMessage),
}
