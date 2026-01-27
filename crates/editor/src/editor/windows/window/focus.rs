use strum_macros::{AsRefStr, EnumIter};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumIter, AsRefStr)]
#[strum(serialize_all = "lowercase")]
pub(crate) enum Focus {
    Search,
    Prompt,
    Window,
    Completion,
    Filetree,
    Locations,
    Snapshots,
}
