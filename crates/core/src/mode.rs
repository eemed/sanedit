use serde::{Deserialize, Serialize};
use strum_macros::{AsRefStr, EnumIter};

#[derive(
    Debug, Default, PartialEq, Eq, Clone, Copy, AsRefStr, EnumIter, Hash, Serialize, Deserialize,
)]
#[strum(serialize_all = "lowercase")]
pub enum Mode {
    #[default]
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
