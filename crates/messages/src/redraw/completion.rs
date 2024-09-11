use std::cmp::max;

use sanedit_core::Choice;
use serde::{Deserialize, Serialize};

use super::{Component, Diffable, Point, Redraw, Size};

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct Completion {
    pub point: Point,
    pub choices: Vec<Choice>,
    pub selected: Option<usize>,
    pub query_len: usize,
}

impl Completion {
    /// Size of completion where everything fits on screen
    pub fn preferred_size(&self) -> Size {
        let width = self.choices.iter().fold(0, |acc, o| {
            // " " + name + " " (+ description + " ")
            let mut len = 0;
            len += 1;
            len += o.to_str_lossy().chars().count();
            len += 1;

            if !o.description().is_empty() {
                len += o.description().chars().count();
                len += 1;
            }
            max(acc, len)
        });
        let height = self.choices.len();
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
