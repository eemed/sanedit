use core::fmt;

use serde::{Deserialize, Serialize};

use super::{Cell, Component, Cursor, Diffable, Redraw};

// TODO optimize size
#[derive(Serialize, Deserialize, PartialEq, Eq, Default, Clone)]
pub struct Window {
    pub cells: Vec<Vec<Cell>>,
    pub cursor: Option<Cursor>,
}

impl Diffable for Window {
    type Diff = Difference;

    fn diff(&self, other: &Self) -> Option<Self::Diff> {
        log::info!(
            "WDiff style len: {}",
            16 * other.cells.len() * other.cells[0].len()
        );
        log::info!(
            "WDiff text len: {}",
            24 * other.cells.len() * other.cells[0].len()
        );
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
