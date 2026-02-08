use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Hash)]
pub enum SnapshotsUpdate {
    Full(Snapshots),
    Selection(Option<usize>),
    Close,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Hash)]
pub struct Snapshots {
    pub points: Vec<SnapshotPoint>,
    /// Index into points of the selected item
    pub selected: usize,
    /// Last saved snapshots id
    pub last_saved_id: usize,
    pub in_focus: bool,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Hash)]
pub struct SnapshotPoint {
    pub title: String,
    pub next: Vec<usize>,
    pub id: usize,
}
