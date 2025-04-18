use core::fmt;

use serde::{Deserialize, Serialize};

use super::{Cell, Component, Cursor, Diffable, Redraw};

#[derive(Serialize, Deserialize, PartialEq, Eq, Default, Clone)]
pub struct Window {
    // TODO optimize cells size
    // => hl spans?
    // => iterator of lines
    // => iter of cells
    pub cells: Vec<Vec<Cell>>,
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

impl From<Window> for Redraw {
    fn from(value: Window) -> Self {
        Redraw::Window(Component::Open(value))
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct Difference {
    window: Window,
}

impl From<Difference> for Redraw {
    fn from(diff: Difference) -> Self {
        Redraw::Window(Component::Update(diff))
    }
}
