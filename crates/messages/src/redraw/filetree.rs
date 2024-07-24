use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::{Component, Diffable, Redraw};

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct Filetree {
    pub items: Vec<FileItem>,
    pub selected: usize,
    pub in_focus: bool,
}

impl From<Filetree> for Redraw {
    fn from(value: Filetree) -> Self {
        Redraw::Filetree(Component::Open(value))
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub enum FileItemKind {
    Directory { expanded: bool },
    File,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct FileItem {
    pub name: PathBuf,
    pub kind: FileItemKind,
    pub level: usize,
}

impl Diffable for Filetree {
    type Diff = Difference;

    fn diff(&self, other: &Self) -> Option<Self::Diff> {
        if self == other {
            return None;
        }

        Some(Difference {
            full: other.clone(),
        })
    }

    fn update(&mut self, diff: Self::Diff) {
        *self = diff.full
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct Difference {
    full: Filetree,
}

impl From<Difference> for Redraw {
    fn from(value: Difference) -> Self {
        Redraw::Filetree(Component::Update(value))
    }
}
