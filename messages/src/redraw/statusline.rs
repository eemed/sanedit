use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct Statusline {
    line: String,
}

impl Statusline {
    pub fn new(line: &str) -> Statusline {
        Statusline { line: line.to_string() }
    }

    pub fn line(&self) -> &str {
        &self.line
    }
}
