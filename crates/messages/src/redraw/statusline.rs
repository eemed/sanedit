use serde::{Deserialize, Serialize};

use super::{Component, Redraw};

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Default, Hash)]
pub struct Statusline {
    pub left: String,
    pub right: String,
}

impl From<Statusline> for Redraw {
    fn from(status: Statusline) -> Self {
        Redraw::Statusline(Component::Update(status))
    }
}

