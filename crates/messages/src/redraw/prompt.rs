use serde::{Deserialize, Serialize};

use super::{choice::Choice, Component, Redraw};

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Hash)]
pub enum Source {
    /// Search prompt
    Search,
    /// A prompt with completions
    Prompt,
    /// A simple prompt / yes no questions
    Simple,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Hash)]
pub struct Prompt {
    pub message: String,
    pub input: String,
    /// Cursor position on input
    pub cursor: usize,
    pub options: Vec<Choice>,
    pub selected: Option<usize>,
    pub source: Source,
    pub max_completions: usize,
    pub is_loading: bool,
}

impl From<Prompt> for Redraw {
    fn from(value: Prompt) -> Self {
        Redraw::Prompt(Component::Update(value))
    }
}
