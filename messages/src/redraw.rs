mod cell;
mod point;
mod prompt;
mod size;
mod statusline;
mod window;

pub use cell::*;
pub use point::*;
pub use size::*;
pub use window::*;
pub use statusline::*;
pub use prompt::*;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub enum Redraw {
    Window(Window),
    Statusline(Statusline),
    Prompt(Prompt),
}
