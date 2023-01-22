mod cell;

use sanedit_buffer::piece_tree::{next_grapheme, PieceTreeSlice};

use crate::editor::{
    buffers::buffer::{Buffer, EOL},
    common::char::{Char, DisplayOptions},
};

pub(crate) use self::cell::Cell;

#[derive(Debug, Default)]
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

    fn draw_trailing_whitespace(&mut self) {}
    fn draw_end_of_buffer(&mut self) {}
    fn draw_cursors(&mut self) {}

    fn redraw(&mut self, buf: &Buffer, opts: &DisplayOptions) {
        let slice = buf.slice(self.offset..);
        let mut pos = 0;
        let mut line = 0;
        let mut col = 0;

        while let Some(grapheme) = next_grapheme(&slice, pos) {
            if line == self.height {
                break;
            }
            let is_eol = EOL::is_eol(&grapheme);
            let ch = Char::new(grapheme, col, opts);
            let width = ch.width();
            let cell = ch.into();
            self.cells[line][col] = cell;
            // TODO advance line + col for width

            //     let mut cells = grapheme_to_cells(&g, v_col, buf.options.tabstop, &win.options.symbols);
        }

        // let symbols = &win.options.symbols;
        // let mut cursor = CellPosition::default();
        // let mut grid = Grid::new(Cell::empty(), width, height);

        // let mut graphemes = buf.graphemes_at(win.offset);
        // let mut grapheme = graphemes.get();

        // let mut c_line = 0;
        // let mut c_col = 0;
        // let mut v_col = 0;

        // while let Some(g) = grapheme {
        //     if c_line == height {
        //         break;
        //     }
        //     let is_eol = is_buf_eol(&buf, &g);
        //     let mut cells = grapheme_to_cells(&g, v_col, buf.options.tabstop, &win.options.symbols);

        //     for cell in cells.into_iter() {
        //         grid[c_line][c_col] = cell;

        //         c_col += 1;
        //         v_col += 1;

        //         if c_col == width {
        //             c_line += 1;
        //             c_col = 0;
        //         }

        //         if c_line == height {
        //             break;
        //         }
        //     }

        //     // c_col != 0 because eol maybe on the last cell and we don't
        //     // want to crate extra empty line
        //     if is_eol && c_col != 0 {
        //         c_line += 1;
        //         c_col = 0;
        //         v_col = 0;
        //     }

        //     grapheme = graphemes.next();

        //     // Set cursor
        //     if graphemes.pos() == win.cursor.pos() {
        //         cursor = CellPosition {
        //             x: c_col,
        //             y: c_line,
        //         };
        //     }
        // }

        // mark_trailing_whitespace(&mut grid, buf, symbols);

        // // create used cell for eof if visible
        // if graphemes.pos() == buf.len() && c_line < height && c_col < width {
        //     grid[c_line][c_col] = Cell::new(" ");
        // }

        // // Fill to end of view
        // while c_line + 1 < height {
        //     c_line += 1;

        //     grid[c_line][0] = Cell::new_unused(&symbols[Symbol::BufferEnd]);
        // }

        // WindowCells {
        //     grid,
        //     cursor_selection: win.cursor.selection_range(),
        //     cursor,
        //     buf_range: win.offset..graphemes.pos(),
        //     style: None,
        // }
    }
}
