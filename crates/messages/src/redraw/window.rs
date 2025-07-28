use core::fmt;
use std::ops::{Deref, DerefMut};

use serde::{Deserialize, Serialize};

use super::{Cell, Component, Cursor, Diffable, Redraw};

#[derive(Serialize, Deserialize, PartialEq, Eq, Default, Clone)]
pub struct Window {
    pub cells: WindowGrid,
    pub cursor: Option<Cursor>,
}

impl Diffable for Window {
    type Diff = Difference;

    fn diff(&self, other: &Self) -> Option<Self::Diff> {
        if self == other {
            return None;
        }

        Some(Difference {
            window: other.clone(),
        })
    }

    fn update(&mut self, diff: Self::Diff) {
        *self = diff.window;
    }
}

impl fmt::Debug for Window {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "===Window===")?;
        for row in self.cells.iter() {
            write!(f, "\"")?;
            for cell in row.iter() {
                write!(f, "{}", cell.text)?;
            }
            writeln!(f, "\"")?;
        }
        write!(f, "==========")?;
        Ok(())
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Default, Clone, Debug)]
pub struct WindowGrid {
    grid: Vec<Vec<Cell>>,
}

impl Deref for WindowGrid {
    type Target = Vec<Vec<Cell>>;

    fn deref(&self) -> &Self::Target {
        &self.grid
    }
}

impl DerefMut for WindowGrid {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.grid
    }
}

impl WindowGrid {
    pub fn new(width: usize, height: usize, cell: Cell) -> WindowGrid {
        Self {
            grid: vec![vec![cell; width]; height],
        }
    }

    pub fn width(&self) -> usize {
        self.grid.get(0).map(|line| line.len()).unwrap_or(0)
    }

    pub fn height(&self) -> usize {
        self.grid.len()
    }
}

impl From<Window> for Redraw {
    fn from(value: Window) -> Self {
        Redraw::Window(Component::Open(value))
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct Difference {
    window: Window,
}

impl From<Difference> for Redraw {
    fn from(diff: Difference) -> Self {
        Redraw::Window(Component::Update(diff))
    }
}
