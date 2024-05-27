use std::cmp::max;

use serde::{Deserialize, Serialize};

use super::{Component, Diffable, Point, Redraw, Size};

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct CompletionOption {
    pub name: String,
    pub description: String,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct Completion {
    pub point: Point,
    pub options: Vec<CompletionOption>,
    pub selected: Option<usize>,
    pub query_len: usize,
}

impl Completion {
    /// Size of completion where everything fits on screen
    pub fn preferred_size(&self) -> Size {
        let width = self.options.iter().fold(0, |acc, o| {
            // name + " " + description
            max(
                acc,
                o.name.chars().count() + 1 + o.description.chars().count(),
            )
        });
        let height = self.options.len();
        Size { width, height }
    }
}

impl Diffable for Completion {
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

impl From<Completion> for Redraw {
    fn from(value: Completion) -> Self {
        Redraw::Completion(Component::Open(value))
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct Difference {
    full: Completion,
}

impl From<Difference> for Redraw {
    fn from(value: Difference) -> Self {
        Redraw::Completion(Component::Update(value))
    }
}
