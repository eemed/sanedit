use std::cmp::min;

use sanedit_messages::redraw::window::Window;

use crate::ui::UIContext;

use super::drawable::{DrawCursor, Drawable, Subgrid};

impl Drawable for Window {
    fn draw(&self, _ctx: &UIContext, mut grid: Subgrid) {
        let width = min(grid.width(), self.width());
        let height = min(grid.height(), self.height());

        grid.clear_all(self.empty_cell().style);

        for ((y, x), cell) in self.used() {
            let y = *y as usize;
            let x = *x as usize;
            if y < height && x < width {
                grid.replace(y, x, cell.clone());
            }
        }
    }

    fn cursor(&self, _ctx: &UIContext) -> DrawCursor {
        match self.cursor {
            Some(cursor) => DrawCursor::Show(cursor),
            None => DrawCursor::Ignore,
        }
    }
}
