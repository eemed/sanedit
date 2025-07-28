use core::fmt;

use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

use super::{Cell, Component, Cursor, Redraw};

#[derive(Serialize, Deserialize, PartialEq, Eq, Default, Clone)]
pub struct Window {
    pub cells: WindowGrid,
    pub cursor: Option<Cursor>,
}

impl fmt::Debug for Window {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "===Window===")?;
        // for row in self.cells.iter() {
        //     write!(f, "\"")?;
        //     for cell in row.iter() {
        //         write!(f, "{}", cell.text)?;
        //     }
        //     writeln!(f, "\"")?;
        // }
        write!(f, "==========")?;
        Ok(())
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Default, Clone, Debug)]
pub struct WindowGrid {
    cells: FxHashMap<(u32, u32), Cell>,
    empty: Cell,
    width: u32,
    height: u32,
}

impl WindowGrid {
    pub fn new(width: usize, height: usize, cell: Cell) -> WindowGrid {
        Self {
            cells: FxHashMap::default(),
            empty: cell,
            width: width as u32,
            height: height as u32,
        }
    }

    pub fn width(&self) -> usize {
        self.width as usize
    }

    pub fn height(&self) -> usize {
        self.height as usize
    }

    pub fn clear(&mut self) {
        self.cells.clear();
    }

    pub fn clear_with(&mut self, cell: Cell) {
        self.empty = cell;
        self.clear();
    }

    pub fn draw(&mut self, y: usize, x: usize, cell: Cell) {
        self.cells.insert((y as u32, x as u32), cell);
    }

    pub fn at(&mut self, y: usize, x: usize) -> &mut Cell {
        let entry = self.cells.entry((y as u32, x as u32));
        entry.or_insert(self.empty.clone())
    }

    pub fn get(&self, y: usize, x: usize) -> &Cell {
        self.cells.get(&(y as u32, x as u32)).unwrap_or(&self.empty)
    }
}

impl From<Window> for Redraw {
    fn from(value: Window) -> Self {
        Redraw::Window(Component::Open(value))
    }
}
