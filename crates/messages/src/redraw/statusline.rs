use serde::{Deserialize, Serialize};

use super::{Component, Diffable, Redraw};

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Default)]
pub struct Statusline {
    pub client_in_focus: bool,
    pub left: String,
    pub right: String,
}

impl Diffable for Statusline {
    type Diff = Difference;

    fn diff(&self, other: &Self) -> Option<Self::Diff> {
        if self == other {
            return None;
        }

        Some(Difference {
            line: other.clone(),
        })
    }

    fn update(&mut self, diff: Self::Diff) {
        *self = diff.line;
    }
}

impl From<Statusline> for Redraw {
    fn from(status: Statusline) -> Self {
        Redraw::Statusline(Component::Open(status))
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct Difference {
    line: Statusline,
}

impl From<Difference> for Redraw {
    fn from(diff: Difference) -> Self {
        Redraw::Statusline(Component::Update(diff))
    }
}
