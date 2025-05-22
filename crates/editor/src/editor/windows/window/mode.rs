use serde::{Deserialize, Serialize};
use strum_macros::{AsRefStr, EnumIter};

#[derive(Debug, PartialEq, Eq, Clone, Copy, AsRefStr, EnumIter, Hash, Serialize, Deserialize)]
#[strum(serialize_all = "lowercase")]
pub(crate) enum Mode {
    Normal,
    Insert,
    Select,
}

impl Mode {
    pub fn statusline(&self) -> &str {
        match self {
            Mode::Normal => "NOR",
            Mode::Insert => "INS",
            Mode::Select => "SEL",
        }
    }
}
