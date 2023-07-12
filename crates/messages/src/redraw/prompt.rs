use serde::{Deserialize, Serialize};

use super::{Component, Diffable, Redraw};

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct Prompt {
    message: String,
    input: String,
    // Cursor position on input
    cursor: usize,
    options: Vec<String>,
    selected: Option<usize>,
}

impl Prompt {
    pub fn new(
        message: &str,
        input: &str,
        cursor: usize,
        options: Vec<&str>,
        selected: Option<usize>,
    ) -> Prompt {
        Prompt {
            message: message.to_string(),
            input: input.to_string(),
            cursor,
            options: options.into_iter().map(|opt| opt.into()).collect(),
            selected,
        }
    }

    pub fn selected(&self) -> Option<usize> {
        self.selected
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn input(&self) -> &str {
        &self.input
    }

    pub fn cursor_in_input(&self) -> usize {
        self.cursor
    }

    pub fn options(&self) -> Vec<&str> {
        self.options.iter().map(|opt| opt.as_str()).collect()
    }
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
