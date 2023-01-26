use serde::{Deserialize, Serialize};

use super::{Cell, Point};

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

    pub fn primary_cursor(&self) -> Point {
        self.primary_cursor
    }
}
