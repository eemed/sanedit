use sanedit_messages::redraw::{
    Cell, IntoCells, Point, Prompt, Statusline, Style, ThemeField, Window,
};

use crate::ui::UIContext;

pub(crate) trait Component {
    fn position(&self, ctx: &UIContext) -> Point;
    fn draw(&mut self, ctx: &UIContext) -> Vec<Vec<Cell>>;
    fn cursor(&self, ctx: &UIContext) -> Option<Point>;
    // fn size(&self) -> Size;
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
        let line = into_cells_with_theme(self.line(), ThemeField::Statusline, ctx);
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
        // TODO merge styles
        let a = ctx.theme.get(ThemeField::PromptDefault.into());
        let b = ctx.theme.get(ThemeField::PromptMessage.into());
        let style = sanedit_messages::redraw::merge_cell_styles(&[a, b]);
        let mut message = into_cells_with_style_no_pad(self.message(), style.unwrap(), ctx);
        // let colon = into_cells_with_theme_no_pad(": ", ThemeField::PromptMessage, ctx);
        // let input = into_cells_with_theme_no_pad(self.input(), ThemeField::PromptUserInput, ctx);

        // message.extend(colon);
        // message.extend(input);

        let mut prompt = vec![message];
        let opts: Vec<Vec<Cell>> = self
            .options()
            .iter()
            .enumerate()
            .map(|(i, opt)| {
                let field = if Some(i) == self.selected() {
                    ThemeField::PromptCompletionSelected
                } else {
                    ThemeField::PromptCompletion
                };
                let cells = into_cells_with_theme(opt, field, ctx);
                cells
            })
            .collect();
        prompt.extend(opts);
        prompt
    }

    fn cursor(&self, ctx: &UIContext) -> Option<Point> {
        let point = self.position(ctx);
        let cursor_col = {
            let input_cells_before_cursor =
                self.input()[..self.cursor_in_input()].into_cells().len();
            let msg_len = self.message().into_cells().len();
            let extra = 2; // ": "
            msg_len + extra + input_cells_before_cursor
        };
        Some(Point {
            x: point.x + cursor_col,
            y: point.y,
        })
    }
}

fn into_cells_with_style_no_pad(string: &str, style: Style, ctx: &UIContext) -> Vec<Cell> {
    let mut cells = string.into_cells();
    cells.iter_mut().for_each(|cell| cell.style = style);
    cells
}

fn into_cells_with_theme(string: &str, themefield: ThemeField, ctx: &UIContext) -> Vec<Cell> {
    let mut cells = string.into_cells();

    while cells.len() < ctx.width {
        cells.push(Cell::default());
    }

    while cells.len() > ctx.width {
        cells.pop();
    }

    cells
        .iter_mut()
        .for_each(|cell| cell.style = ctx.theme.get(themefield.into()).unwrap_or(Style::default()));
    cells
}
