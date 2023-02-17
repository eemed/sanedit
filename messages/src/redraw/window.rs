use serde::{Deserialize, Serialize};

use super::{Cell, Point, Redraw};

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
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

    pub fn patch(&mut self, patch: WindowPatch) {
        todo!()
    }

    pub fn diff(&self, other: &Window) -> WindowPatch {
        todo!()
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
pub struct WindowPatch(WindowUpdateInner);

impl WindowPatch {
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
enum WindowUpdateInner {}
