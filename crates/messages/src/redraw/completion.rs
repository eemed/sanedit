use serde::{Deserialize, Serialize};

use super::{Diffable, Redraw};

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct Completion {
    pub options: Vec<String>,
    pub selected: Option<usize>,
}

impl Diffable for Completion {
    type Diff = Difference;

    fn diff(&self, other: Self) -> Self::Diff {
        if self == other {
            return None;
        }

        Some(Difference {
            full: other.clone(),
        })
    }

    fn update(&mut self, diff: Self::Diff) {
        todo!()
    }
}

impl Completion {
    pub fn diff(&self, other: &Completion) -> Option<Difference> {
        if self == other {
            return None;
        }

        Some(Difference {
            full: other.clone(),
        })
    }
}

impl From<Completion> for Redraw {
    fn from(value: Completion) -> Self {
        Redraw::Completion(value)
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct Difference {
    full: Completion,
}

impl From<Difference> for Redraw {
    fn from(value: Difference) -> Self {
        Redraw::CompletionUpdate(value)
    }
}
