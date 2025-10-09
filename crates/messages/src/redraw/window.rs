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
    cells: Vec<Vec<Cell>>,
    empty: Cell,
    width: u32,
    height: u32,
}

impl Window {
    pub fn new(width: usize, height: usize, cell: Cell) -> Window {
        Self {
            cursor: None,
            cells: vec![Vec::with_capacity(width); height],
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
        for line in &mut self.cells {
            line.clear();
        }
    }

    pub fn clear_with(&mut self, cell: Cell) {
        self.empty = cell;
        self.clear();
    }

    pub fn draw(&mut self, y: usize, x: usize, cell: Cell) {
        let line = &mut self.cells[y];
        while line.len() <= x {
            line.push(self.empty.clone());
        }

        line[x] = cell;
    }

    pub fn at(&mut self, y: usize, x: usize) -> &mut Cell {
        let line = &mut self.cells[y];
        while line.len() <= x {
            line.push(self.empty.clone());
        }

        &mut line[x]
    }

    pub fn get(&self, y: usize, x: usize) -> &Cell {
        self.cells
            .get(y)
            .and_then(|line| line.get(x))
            .unwrap_or(&self.empty)
    }

    pub fn used<'a>(&'a self) -> Iter<'a> {
        Iter {
            cells: &self.cells,
            line: 0,
            col: 0,
        }
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

pub struct Iter<'a> {
    cells: &'a Vec<Vec<Cell>>,
    line: usize,
    col: usize,
}

impl<'a> Iterator for Iter<'a> {
    type Item = (usize, usize, &'a Cell);

    fn next(&mut self) -> Option<Self::Item> {
        let line = self.cells.get(self.line)?;
        match line.get(self.col) {
            Some(cell) => {
                let item = (self.line, self.col, cell);
                self.col += 1;
                Some(item)
            }
            None => {
                self.col = 0;
                self.line += 1;
                self.next()
            }
        }
    }
}
