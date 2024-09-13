use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum Severity {
    #[default]
    Hint,
    Info,
    Warn,
    Error,
}
