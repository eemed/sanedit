use sanedit_messages::redraw::{
    Cell, IntoCells, Point, Prompt, Size, Statusline, Style, ThemeField, Window,
};

use crate::ui::UIContext;

pub(crate) trait Component {
    fn position(&self, ctx: &UIContext) -> Point;
    fn draw(&self, ctx: &UIContext) -> Vec<Vec<Cell>>;
    fn cursor(&self, ctx: &UIContext) -> Option<Point>;
    fn size(&self, ctx: &UIContext) -> Size;
}

impl Component for Window {
    fn position(&self, ctx: &UIContext) -> Point {
        Point { x: 0, y: 1 }
    }

    fn draw(&self, ctx: &UIContext) -> Vec<Vec<Cell>> {
        self.cells().clone()
    }

    fn cursor(&self, ctx: &UIContext) -> Option<Point> {
        let cursor = self.primary_cursor();
        let pos = self.position(ctx);
        Some(cursor + pos)
    }

    fn size(&self, ctx: &UIContext) -> Size {
        Size {
            width: ctx.width,
            height: ctx.height - 1,
        }
    }
}

impl Component for Statusline {
    fn position(&self, ctx: &UIContext) -> Point {
        Point { x: 0, y: 0 }
    }

    fn draw(&self, ctx: &UIContext) -> Vec<Vec<Cell>> {
        let line = into_cells_with_theme_pad(self.line(), &ThemeField::Statusline, ctx);

        vec![line]
    }

    fn cursor(&self, ctx: &UIContext) -> Option<Point> {
        None
    }

    fn size(&self, ctx: &UIContext) -> Size {
        Size {
            width: ctx.width,
            height: 1,
        }
    }
}

impl Component for Prompt {
    fn position(&self, ctx: &UIContext) -> Point {
        Point { x: 0, y: 0 }
    }

    fn draw(&self, ctx: &UIContext) -> Vec<Vec<Cell>> {
        let style = ctx
            .theme
            .get(ThemeField::PromptDefault.into())
            .unwrap_or(Style::default());
        let msg_style = {
            let msg = ctx
                .theme
                .get(ThemeField::PromptMessage.into())
                .unwrap_or(Style::default());
            sanedit_messages::redraw::merge_cell_styles(&[style, msg])
        };

        let input_style = {
            let input = ctx
                .theme
                .get(ThemeField::PromptUserInput.into())
                .unwrap_or(Style::default());
            sanedit_messages::redraw::merge_cell_styles(&[style, input])
        };

        let mut message = into_cells_with_style(self.message(), msg_style, ctx);
        let colon = into_cells_with_style(": ", msg_style, ctx);
        let input = into_cells_with_style(self.input(), input_style, ctx);
        message.extend(colon);
        message.extend(input);

        pad_line(&mut message, style, ctx);

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
                let cells = into_cells_with_theme_pad(opt, &field, ctx);
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

    fn size(&self, ctx: &UIContext) -> Size {
        todo!()
        // Size { width: ctx.width, height: 1 }
    }
}

fn into_cells_with_style(string: &str, style: Style, ctx: &UIContext) -> Vec<Cell> {
    let mut cells = string.into_cells();
    cells.iter_mut().for_each(|cell| cell.style = style);
    cells
}

fn into_cells_with_theme(string: &str, themefield: &ThemeField, ctx: &UIContext) -> Vec<Cell> {
    let mut cells = string.into_cells();
    let style = ctx.style(&themefield);
    cells.iter_mut().for_each(|cell| cell.style = style);
    cells
}

fn into_cells_with_theme_pad(string: &str, themefield: &ThemeField, ctx: &UIContext) -> Vec<Cell> {
    let mut cells = into_cells_with_theme(string, themefield, ctx);
    pad_line(&mut cells, ctx.style(themefield), ctx);
    cells
}

fn pad_line(cells: &mut Vec<Cell>, style: Style, ctx: &UIContext) {
    while cells.len() < ctx.width {
        cells.push(Cell::with_style(style.clone()));
    }

    while cells.len() > ctx.width {
        cells.pop();
    }
}
