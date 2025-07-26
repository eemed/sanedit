use std::fmt::Display;

use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

use super::Style;

pub trait IntoCells {
    fn into_cells(self) -> Vec<Cell>;
}

impl IntoCells for &str {
    fn into_cells(self) -> Vec<Cell> {
        self.chars().map(Cell::from).collect()
    }
}

const EMPTY: SmolStr = SmolStr::new_static(" ");

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct Cell {
    pub text: SmolStr,
    pub style: Style,
}

impl Cell {
    pub fn new<A: AsRef<str>>(item: A, style: Style) -> Cell {
        Cell {
            text: SmolStr::from(item.as_ref()),
            style,
        }
    }

    pub fn new_char(ch: char, style: Style) -> Cell {
        let mut buf = [0u8; 4];
        let string = ch.encode_utf8(&mut buf);
        Cell {
            text: SmolStr::from(string),
            style,
        }
    }

    pub fn with_style(style: Style) -> Cell {
        Cell { text: EMPTY, style }
    }

    pub fn is_blank(&self) -> bool {
        self.text.chars().all(char::is_whitespace)
    }
}

impl Default for Cell {
    fn default() -> Self {
        Cell {
            text: EMPTY,
            style: Style::default(),
        }
    }
}

impl From<&str> for Cell {
    fn from(string: &str) -> Self {
        Cell {
            text: SmolStr::from(string),
            style: Style::default(),
        }
    }
}

impl From<char> for Cell {
    fn from(ch: char) -> Self {
        let mut buf = [0u8; 4];
        let string = ch.encode_utf8(&mut buf);
        Cell {
            text: SmolStr::from(string),
            style: Style::default(),
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
