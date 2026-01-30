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
    pub selected: usize,
    pub in_focus: bool,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Hash)]
pub struct SnapshotPoint {
    pub title: String,
    pub children: Vec<usize>,
}
