use serde::{Deserialize, Serialize};

use super::Redraw;

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

    pub fn update(&mut self, diff: PromptDiff) {
        *self = diff.prompt;
    }

    pub fn diff(&self, other: &Prompt) -> Option<PromptDiff> {
        if self == other {
            return None;
        }

        Some(PromptDiff {
            prompt: other.clone(),
        })
    }
}

impl From<Prompt> for Redraw {
    fn from(value: Prompt) -> Self {
        Redraw::Prompt(value)
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct PromptDiff {
    prompt: Prompt,
}


impl From<PromptDiff> for Redraw {
    fn from(diff: PromptDiff) -> Self {
        Redraw::PromptUpdate(diff)
    }
}
