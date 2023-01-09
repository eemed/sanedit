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
    content: Option<SmolStr>,
}

impl Display for Cell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.content {
            Some(content) => f.write_str(content),
            None => f.write_str(""),
        }
    }
}

impl Default for Cell {
    fn default() -> Self {
        Cell { content: None }
    }
}

pub(crate) trait Component {
    fn position(&self) -> Point;
    fn draw(&mut self) -> Vec<Vec<Cell>>;
    // fn size(&self) -> Size;
    // fn styles(&mut self) -> Vec<Style>;
}
