use sanedit_core::Severity;
use serde::{Deserialize, Serialize};

use super::Redraw;

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct StatusMessage {
    pub severity: Severity,
    pub message: String,
}

impl From<StatusMessage> for Redraw {
    fn from(msg: StatusMessage) -> Self {
        Redraw::StatusMessage(msg)
    }
}
