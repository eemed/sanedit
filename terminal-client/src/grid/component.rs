use sanedit_messages::redraw::{
    Cell, IntoCells, Point, Prompt, Statusline, Style, ThemeField, Window,
};

use crate::ui::UIContext;

pub(crate) trait Component {
    fn position(&self, ctx: &UIContext) -> Point;
    fn draw(&mut self, ctx: &UIContext) -> Vec<Vec<Cell>>;
    fn cursor(&self, ctx: &UIContext) -> Option<Point>;
    // fn size(&self) -> Size;
    // fn styles(&mut self) -> Vec<Style>;
}

impl Component for Window {
    fn position(&self, ctx: &UIContext) -> Point {
        Point { x: 0, y: 1 }
    }

    fn draw(&mut self, ctx: &UIContext) -> Vec<Vec<Cell>> {
        self.cells().clone()
    }

    fn cursor(&self, ctx: &UIContext) -> Option<Point> {
        let cursor = self.primary_cursor();
        let pos = self.position(ctx);
        Some(cursor + pos)
    }
}

impl Component for Statusline {
    fn position(&self, ctx: &UIContext) -> Point {
        Point { x: 0, y: 0 }
    }

    fn draw(&mut self, ctx: &UIContext) -> Vec<Vec<Cell>> {
        let mut line = self.line().into_cells();
        while line.len() < ctx.width {
            line.push(Cell::default());
        }
        while line.len() > ctx.width {
            line.pop();
        }

        line.iter_mut().for_each(|cell| {
            cell.style = ctx
                .theme
                .get(ThemeField::Statusline.into())
                .unwrap_or(Style::default());
        });

        vec![line]
    }

    fn cursor(&self, ctx: &UIContext) -> Option<Point> {
        None
    }
}

impl Component for Prompt {
    fn position(&self, ctx: &UIContext) -> Point {
        Point { x: 0, y: 0 }
    }

    fn draw(&mut self, ctx: &UIContext) -> Vec<Vec<Cell>> {
        let line = format!("{}: {}", self.message(), self.input());
        let mut prompt = vec![line.into_cells()];
        let opts: Vec<Vec<Cell>> = self.options().iter().map(|opt| opt.into_cells()).collect();
        prompt.extend(opts);
        prompt
    }

    fn cursor(&self, ctx: &UIContext) -> Option<Point> {
        let point = self.position(ctx);
        let cursor_col = {
            let input_cells_before_cursor =
                self.input()[..self.cursor_in_input()].into_cells().len();
            let msg_len = self.message().into_cells().len();
            let extra = 2; // " :"
            msg_len + extra + input_cells_before_cursor
        };
        Some(Point {
            x: point.x + cursor_col,
            y: point.y,
        })
    }
}
