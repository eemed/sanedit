use std::fmt::Display;

use smol_str::SmolStr;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: usize,
    pub y: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Size {
    width: usize,
    height: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Cell {
    content: SmolStr,
}

impl From<&str> for Cell {
    fn from(value: &str) -> Self {
        Cell {
            content: SmolStr::from(value),
        }
    }
}

impl PartialEq<str> for Cell {
    fn eq(&self, other: &str) -> bool {
        self.content == other
    }
}

impl Display for Cell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.content)
    }
}

impl Default for Cell {
    fn default() -> Self {
        Cell { content: "".into() }
    }
}

pub(crate) trait Component {
    fn position(&self) -> Point;
    fn draw(&mut self) -> Vec<Vec<Cell>>;
    // fn size(&self) -> Size;
    // fn styles(&mut self) -> Vec<Style>;
}

impl Component for Vec<Vec<Cell>> {
    fn position(&self) -> Point {
        Point { x: 0, y: 0 }
    }

    fn draw(&mut self) -> Vec<Vec<Cell>> {
        self.clone()
    }
}
