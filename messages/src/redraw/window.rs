use serde::{Deserialize, Serialize};

use super::{Cell, Point, Redraw};

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Default, Clone)]
pub struct Window {
    cells: Vec<Vec<Cell>>,
    primary_cursor: Point,
}

impl Window {
    pub fn new(cells: Vec<Vec<Cell>>, primary_cursor: Point) -> Window {
        Window {
            cells,
            primary_cursor,
        }
    }

    pub fn cells(&self) -> &Vec<Vec<Cell>> {
        &self.cells
    }

    pub fn draw(&self) -> &Vec<Vec<Cell>> {
        &self.cells
    }

    pub fn update(&mut self, diff: WindowDiff) {
        *self = diff.window;
    }

    pub fn diff(&self, other: &Window) -> Option<WindowDiff> {
        if self == other {
            return None;
        }

        Some(WindowDiff {
            window: other.clone(),
        })
    }

    pub fn primary_cursor(&self) -> Point {
        self.primary_cursor
    }
}

impl From<Window> for Redraw {
    fn from(value: Window) -> Self {
        Redraw::Window(value)
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct WindowDiff {
    window: Window,
}
