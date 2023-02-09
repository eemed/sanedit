use serde::{Deserialize, Serialize};

use super::Cell;

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct Statusline {
    line: Vec<Cell>,
}

impl Statusline {
    pub fn new(line: Vec<Cell>) -> Statusline {
        Statusline { line }
    }

    pub fn line(&self) -> &Vec<Cell> {
        &self.line
    }
}
