mod cell;

use sanedit_buffer::piece_tree::next_grapheme;

use crate::common::char::{Char, DisplayOptions};
use crate::editor::buffers::buffer::{Buffer, EOL};

pub(crate) use self::cell::Cell;

#[derive(Debug)]
pub(crate) struct View {
    offset: usize,
    cells: Vec<Vec<Cell>>,
    width: usize,
    height: usize,
}

impl View {
    pub fn new(width: usize, height: usize) -> View {
        View {
            offset: 0,
            cells: vec![vec![Cell::default(); width]; height],
            width,
            height,
        }
    }

    pub fn clear(&mut self) {
        let width = self.width();
        let height = self.height();
        *self = View::new(width, height);
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    /// Advance line and col in the grid by amount
    fn advance(&mut self, line: &mut usize, col: &mut usize, amount: usize) {
        for _ in 0..amount {
            *col += 1;

            if *col == self.width {
                *line += 1;
                *col = 0;
            }

            if *line == self.height {
                break;
            }
        }
    }

    fn draw_trailing_whitespace(&mut self) {}
    fn draw_end_of_buffer(&mut self) {}
    fn draw_cursors(&mut self) {}
    fn draw_cells(&mut self, buf: &Buffer, opts: &DisplayOptions) {
        let slice = buf.slice(self.offset..);
        let mut pos = 0;
        let mut line = 0;
        let mut col = 0;

        while let Some(grapheme) = next_grapheme(&slice, pos) {
            if line == self.height {
                break;
            }
            let grapheme_len = grapheme.len();
            let is_eol = EOL::is_eol(&grapheme);
            let ch = Char::new(grapheme, col, opts);
            let ch_width = ch.width();
            let cell = ch.into();
            self.cells[line][col] = cell;

            self.advance(&mut line, &mut col, ch_width);

            // c_col != 0 because eol maybe on the last cell and we don't
            // want to crate extra empty line
            if is_eol && col != 0 {
                line += 1;
                col = 0;
            }

            pos += grapheme_len;
        }
    }

    pub fn redraw(&mut self, buf: &Buffer, opts: &DisplayOptions) {
        self.draw_cells(buf, opts);
        self.draw_cursors();
        self.draw_end_of_buffer();
        self.draw_trailing_whitespace();
    }
}

impl From<&View> for Vec<Vec<String>> {
    fn from(view: &View) -> Self {
        let mut grid = vec![vec![String::new(); view.width()]; view.height()];

        for (line, row) in view.cells.iter().enumerate() {
            for (col, cell) in row.iter().enumerate() {
                grid[line][col] = cell.char().display().to_string();
            }
        }

        grid
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn tabs() {
        let width = 80;
        let opts = DisplayOptions::default();
        let mut buf = Buffer::new();
        buf.append("\tHello\tWorld");

        let mut view = View::new(width, 1);
        view.redraw(&buf, &opts);

        println!("{}", "-".repeat(width));
        for row in &view.cells {
            for cell in row {
                print!("{}", cell.char().display());
            }
            println!("");
        }
        println!("{}", "-".repeat(width));
    }
}
