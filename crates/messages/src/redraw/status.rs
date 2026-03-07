pub use sanedit_core::{Focus, Mode};
use serde::{Deserialize, Serialize};

use super::Redraw;

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Default, Hash)]
pub struct Status {
    pub buffer: String,
    pub buffer_modified: bool,
    pub buffer_read_only: bool,
    pub mode: Mode,
    pub focus: Focus,
    pub cursor_percentage: usize,
    pub macro_recording: bool,
    pub pressed_keys: String,
}

impl From<Status> for Redraw {
    fn from(status: Status) -> Self {
        Redraw::Status(status)
    }
}
