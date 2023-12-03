use serde::{Deserialize, Serialize};

use super::{Component, Diffable, Redraw};

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub enum PromptType {
    Oneline,
    Overlay,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct Prompt {
    pub message: String,
    pub input: String,
    /// Cursor position on input
    pub cursor: usize,
    pub options: Vec<String>,
    pub selected: Option<usize>,
    pub ptype: PromptType,
    pub max_completions: usize,
}

impl Diffable for Prompt {
    type Diff = Difference;

    fn diff(&self, other: &Self) -> Option<Self::Diff> {
        if self == other {
            return None;
        }

        Some(Difference {
            prompt: other.clone(),
        })
    }

    fn update(&mut self, diff: Self::Diff) {
        *self = diff.prompt;
    }
}

impl From<Prompt> for Redraw {
    fn from(value: Prompt) -> Self {
        Redraw::Prompt(Component::Open(value))
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct Difference {
    prompt: Prompt,
}

impl From<Difference> for Redraw {
    fn from(diff: Difference) -> Self {
        Redraw::Prompt(Component::Update(diff))
    }
}
