use serde::{Deserialize, Serialize};

use super::Redraw;

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Default)]
pub struct Statusline {
    line: String,
}

impl Statusline {
    pub fn new(line: &str) -> Statusline {
        Statusline {
            line: line.to_string(),
        }
    }

    pub fn line(&self) -> &str {
        &self.line
    }

    pub fn update(&mut self, diff: StatuslineDiff) {
        *self = diff.line;
    }

    pub fn diff(&self, other: &Statusline) -> Option<StatuslineDiff> {
        if self.line == other.line {
            return None;
        }

        Some(StatuslineDiff {
            line: other.clone(),
        })
    }
}

impl From<Statusline> for Redraw {
    fn from(value: Statusline) -> Self {
        Redraw::Statusline(value)
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct StatuslineDiff {
    line: Statusline,
}
