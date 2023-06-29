mod component;

use core::fmt;

use sanedit_messages::redraw::{Cell, Cursor, Prompt, StatusMessage, Statusline, Window};

use crate::ui::UIContext;

pub(crate) use self::component::Component;

pub(crate) struct Grid {
    pub window: Window,
    pub statusline: Statusline,
    pub prompt: Option<Prompt>,
    pub msg: Option<StatusMessage>,

    cells: Vec<Vec<Cell>>,
    cursor: Cursor,
}

impl Grid {
    pub fn new(width: usize, height: usize) -> Grid {
        Grid {
            window: Window::default(),
            statusline: Statusline::default(),
            prompt: None,
            msg: None,
            cells: vec![vec![Cell::default(); width]; height],
            cursor: Cursor::default(),
        }
    }

    pub fn resize(&mut self, width: usize, height: usize) {
        self.cells = vec![vec![Cell::default(); width]; height];
    }

    pub fn clear(&mut self) {
        for row in self.cells.iter_mut() {
            for cell in row.iter_mut() {
                *cell = Cell::default();
            }
        }
    }

    fn draw_component(
        comp: &dyn Component,
        ctx: &UIContext,
        cursor: &mut Cursor,
        cells: &mut Vec<Vec<Cell>>,
    ) {
        if let Some(cur) = comp.cursor(ctx) {
            *cursor = cur;
        }

        let top_left = comp.position(ctx);
        let grid = comp.draw(ctx);
        for (line, row) in grid.into_iter().enumerate() {
            for (col, cell) in row.into_iter().enumerate() {
                let x = top_left.x + col;
                let y = top_left.y + line;
                if x < ctx.width && y < ctx.height {
                    cells[y][x] = cell;
                }
            }
        }
    }

    pub fn draw(&mut self, ctx: &UIContext) -> (&Vec<Vec<Cell>>, Cursor) {
        self.clear();

        Self::draw_component(&self.window, ctx, &mut self.cursor, &mut self.cells);
        Self::draw_component(&self.statusline, ctx, &mut self.cursor, &mut self.cells);

        if let Some(ref prompt) = self.prompt {
            Self::draw_component(prompt, ctx, &mut self.cursor, &mut self.cells);
        }
        if let Some(ref msg) = self.msg {
            Self::draw_component(msg, ctx, &mut self.cursor, &mut self.cells);
        }

        (&self.cells, self.cursor)
    }
}

impl fmt::Debug for Grid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "===Grid===")?;
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
