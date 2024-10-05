use serde::{Deserialize, Serialize};
use strum_macros::AsRefStr;

#[derive(
    Clone, Debug, Copy, Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, AsRefStr,
)]
#[strum(serialize_all = "lowercase")]
pub enum Severity {
    #[default]
    Hint,
    Info,
    Warn,
    Error,
}
