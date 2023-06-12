mod component;

use core::fmt;

use sanedit_messages::redraw::{
    Cell, Cursor, CursorShape, Point, Prompt, StatusMessage, Statusline, Window,
};

use crate::ui::UIContext;

pub(crate) use self::component::Component;

pub(crate) struct Grid {
    pub window: Window,
    pub statusline: Statusline,
    pub prompt: Option<Prompt>,
    pub msg: Option<StatusMessage>,

    cells: Vec<Vec<Cell>>,
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

impl Grid {
    pub fn new(width: usize, height: usize) -> Grid {
        Grid {
            window: Window::default(),
            statusline: Statusline::default(),
            prompt: None,
            msg: None,
            cells: vec![vec![Cell::default(); width]; height],
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

    pub fn draw(&mut self, ctx: &UIContext) -> (&Vec<Vec<Cell>>, Cursor) {
        self.clear();

        let mut cursor = Cursor::default();
        let components: Vec<&dyn Component> = {
            let mut comps: Vec<&dyn Component> = Vec::new();
            comps.push(&self.window);
            comps.push(&self.statusline);
            if let Some(ref prompt) = self.prompt {
                comps.push(prompt);
            }
            if let Some(ref msg) = self.msg {
                comps.push(msg);
            }
            comps
        };

        for component in components.into_iter() {
            if let Some(cur) = component.cursor(ctx) {
                cursor = cur;
            }

            let top_left = component.position(ctx);
            let grid = component.draw(ctx);
            for (line, row) in grid.into_iter().enumerate() {
                for (col, cell) in row.into_iter().enumerate() {
                    let x = top_left.x + col;
                    let y = top_left.y + line;
                    if x < ctx.width && y < ctx.height {
                        self.cells[y][x] = cell;
                    }
                }
            }
        }

        // log::info!("{:?}", self);
        (&self.cells, cursor)
    }
}
