mod component;

use core::fmt;
use std::mem;

use sanedit_messages::redraw::{Cell, Point};

pub(crate) use self::component::Component;

pub(crate) struct Grid {
    width: usize,
    height: usize,
    components: Vec<Box<dyn Component>>,
}

impl fmt::Debug for Grid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Grid")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("component_count", &self.components.len())
            .finish()
    }
}

impl Grid {
    pub fn new(width: usize, height: usize) -> Grid {
        Grid {
            width,
            height,
            components: Vec::new(),
        }
    }

    pub fn push_component(&mut self, comp: impl Component + 'static) {
        self.components.push(Box::new(comp));
    }

    pub fn draw(&mut self) -> (Vec<Vec<Cell>>, Point) {
        let mut cursor = Point::default();
        let mut canvas: Vec<Vec<Cell>> = vec![vec![Cell::default(); self.width]; self.height];
        let components = mem::take(&mut self.components);

        for mut component in components.into_iter() {
            if let Some(cur) = component.cursor() {
                cursor = cur;
            }

            let top_left = component.position();
            let grid = component.draw();
            for (line, row) in grid.into_iter().enumerate() {
                for (col, cell) in row.into_iter().enumerate() {
                    let x = top_left.x + col;
                    let y = top_left.y + line;
                    if x < self.width && y < self.height {
                        canvas[y][x] = cell;
                    }
                }
            }
        }

        (canvas, cursor)
    }
}
