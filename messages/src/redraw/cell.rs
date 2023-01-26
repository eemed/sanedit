use std::fmt::Display;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Debug, Default, Clone)]
pub struct Cell {
    text: String,
}

impl Cell {
    pub fn as_str(&self) -> &str {
        &self.text
    }
}

impl From<&str> for Cell {
    fn from(string: &str) -> Self {
        Cell {
            text: string.to_string(),
        }
    }
}

impl PartialEq<str> for Cell {
    fn eq(&self, other: &str) -> bool {
        self.text == other
    }
}

impl Display for Cell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.text)
    }
}
