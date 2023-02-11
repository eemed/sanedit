use serde::{Deserialize, Serialize};

use super::Cell;

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct Prompt {
    message: String,
    input: String,
    // Cursor position on input
    cursor: usize,
    options: Vec<String>,
}

impl Prompt {
    pub fn new(message: &str, input: &str, cursor: usize, options: Vec<&str>) -> Prompt {
        Prompt {
            message: message.to_string(),
            input: input.to_string(),
            cursor,
            options: options.into_iter().map(|opt| opt.into()).collect(),
        }
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
