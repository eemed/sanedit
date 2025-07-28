use core::fmt;

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
    v: Vec<Cell>,
    col: Vec<u32>,
    row: Vec<u32>,

    empty: Cell,
    width: u32,
    height: u32,
}

impl WindowGrid {
    pub fn new(width: usize, height: usize, cell: Cell) -> WindowGrid {
        Self {
            v: vec![],
            col: vec![],
            row: vec![],
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
        self.v.clear();
        self.col.clear();
        self.row.clear();
    }

    pub fn clear_with(&mut self, cell: Cell) {
        self.empty = cell;
        self.clear();
    }

    pub fn draw(&mut self, y: usize, x: usize, cell: Cell) {
        self.v.push(cell);
        self.col.push(x as u32);
        self.row.push(y as u32);
    }

    pub fn at(&mut self, y: usize, x: usize) -> &mut Cell {
        todo!()
        // let y = y as u32;
        // let x = x as u32;
        // for (i, ys) in self.row.iter().enumerate() {
        //     if *ys == y && self.col[i] == x {
        //         return &mut self.v[i];
        //     }
        // }
    }

    pub fn get(&self, y: usize, x: usize) -> &Cell {
        todo!()
    }
}

impl From<Window> for Redraw {
    fn from(value: Window) -> Self {
        Redraw::Window(Component::Open(value))
    }
}
