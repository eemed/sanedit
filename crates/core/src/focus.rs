use serde::{Deserialize, Serialize};
use strum_macros::{AsRefStr, EnumIter};

#[derive(
    Debug, Default, Deserialize, Serialize, Clone, Copy, PartialEq, Eq, Hash, EnumIter, AsRefStr,
)]
#[strum(serialize_all = "lowercase")]
pub enum Focus {
    #[default]
    Window,
    Search,
    Prompt,
    Completion,
    Filetree,
    Locations,
    Snapshots,
}
