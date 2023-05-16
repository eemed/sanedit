mod component;

use core::fmt;

use sanedit_messages::redraw::{Cell, Cursor, CursorShape, Point, Prompt, Statusline, Window};

use crate::ui::UIContext;

pub(crate) use self::component::Component;

pub(crate) struct Grid {
    pub window: Window,
    pub statusline: Statusline,
    pub prompt: Option<Prompt>,

    cells: Vec<Vec<Cell>>,
}

impl fmt::Debug for Grid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Grid").finish_non_exhaustive()
    }
}

impl Grid {
    pub fn new() -> Grid {
        Grid {
            window: Window::default(),
            statusline: Statusline::default(),
            prompt: None,
            cells: vec![vec![]],
        }
    }

    pub fn reset_grid(&mut self, width: usize, height: usize) {
        let h = self.cells.len();
        let w = self.cells.get(0).map(|row| row.len()).unwrap_or(0);
        if w != width || height != h {
            self.cells = vec![vec![Cell::default(); width]; height];
            return;
        }

        for row in self.cells.iter_mut() {
            for cell in row.iter_mut() {
                *cell = Cell::default();
            }
        }
    }

    pub fn draw(&mut self, ctx: &UIContext) -> (&Vec<Vec<Cell>>, Cursor) {
        let mut cursor = Cursor::default();
        self.reset_grid(ctx.width, ctx.height);
        let components: Vec<&dyn Component> = {
            let mut comps: Vec<&dyn Component> = Vec::new();
            comps.push(&self.window);
            comps.push(&self.statusline);
            if let Some(ref prompt) = self.prompt {
                comps.push(prompt);
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

        // log::info!("{:?}", self.window);
        (&self.cells, cursor)
    }
}
