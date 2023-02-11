mod cell;
mod color;
mod point;
mod prompt;
mod size;
mod statusline;
mod style;
mod text_style;
mod theme;
mod window;

pub use cell::*;
pub use color::*;
pub use point::*;
pub use prompt::*;
pub use size::*;
pub use statusline::*;
pub use style::*;
pub use text_style::*;
pub use theme::*;
pub use window::*;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub enum Redraw {
    Window(Window),
    Statusline(Statusline),
    Prompt(Prompt),
}
