use serde::{Deserialize, Serialize};

use super::Redraw;

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct Statusline {
    line: String,
}

impl Statusline {
    pub fn new(line: &str) -> Statusline {
        Statusline { line: line.to_string() }
    }

    pub fn line(&self) -> &str {
        &self.line
    }
}

impl From<Statusline> for Redraw {
    fn from(value: Statusline) -> Self {
        Redraw::Statusline(value)
    }
}
