use serde::{Deserialize, Serialize};

use super::Cell;

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct Prompt {
    prompt: Vec<Cell>,
    // Cursor position on prompt
    cursor: usize,

    options: Vec<Vec<Cell>>,
}

impl Prompt {
    pub fn new(prompt: Vec<Cell>, cursor: usize, options: Vec<Vec<Cell>>) -> Prompt {
        Prompt {
            prompt,
            cursor,
            options,
        }
    }

    pub fn prompt(&self) -> &Vec<Cell> {
        &self.prompt
    }

    pub fn cursor_x(&self) -> usize {
        self.cursor
    }

    pub fn options(&self) -> &Vec<Vec<Cell>> {
        &self.options
    }
}
