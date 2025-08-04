use std::collections::BTreeMap;

use super::{Cell, Component, Cursor, Redraw};
use serde::{Deserialize, Serialize};
//
// #[derive(Serialize, Deserialize, PartialEq, Eq, Default, Clone)]
// pub struct Window {
//     pub cells: WindowGrid,
// }

// impl fmt::Debug for Window {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         writeln!(f, "===Window===")?;
//         // for row in self.cells.iter() {
//         //     write!(f, "\"")?;
//         //     for cell in row.iter() {
//         //         write!(f, "{}", cell.text)?;
//         //     }
//         //     writeln!(f, "\"")?;
//         // }
//         write!(f, "==========")?;
//         Ok(())
//     }
// }

#[derive(Serialize, Deserialize, PartialEq, Eq, Default, Clone, Debug, Hash)]
pub struct Window {
    pub cursor: Option<Cursor>,
    cells: BTreeMap<(u32, u32), Cell>,
    empty: Cell,
    width: u32,
    height: u32,
}

impl Window {
    pub fn new(width: usize, height: usize, cell: Cell) -> Window {
        Self {
            cursor: None,
            cells: BTreeMap::default(),
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

    pub fn used(&self) -> std::collections::btree_map::Iter<'_, (u32, u32), Cell> {
        self.cells.iter()
    }

    pub fn empty_cell(&self) -> Cell {
        self.empty.clone()
    }
}

impl From<Window> for Redraw {
    fn from(value: Window) -> Self {
        Redraw::Window(Component::Update(value))
    }
}
