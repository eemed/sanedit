use serde::{Deserialize, Serialize};
use strum_macros::AsRefStr;

#[derive(
    Clone, Debug, Copy, Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, AsRefStr,
)]
pub enum Severity {
    #[default]
    #[strum(serialize = "hint")]
    Hint,
    #[strum(serialize = "info")]
    Info,
    #[strum(serialize = "warn")]
    Warn,
    #[strum(serialize = "error")]
    Error,
}
