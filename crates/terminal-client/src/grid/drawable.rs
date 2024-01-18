use std::cmp::min;

use sanedit_messages::redraw::{
    Cursor, CursorShape, IntoCells, Point, Severity, StatusMessage, Statusline, ThemeField, Window,
};

use crate::{grid::ccell::format_option, ui::UIContext};

use super::{
    border::{draw_border, Border},
    ccell::{
        center_pad, into_cells_with_style, into_cells_with_style_pad,
        into_cells_with_theme_pad_with, pad_line, put_line, set_style, size, CCell,
    },
    prompt::{CustomPrompt, PromptStyle},
};

pub(crate) trait Drawable {
    fn draw(&self, ctx: &UIContext, cells: &mut [&mut [CCell]]);
    fn cursor(&self, ctx: &UIContext) -> Option<Cursor>;
}

impl Drawable for Window {
    fn draw(&self, ctx: &UIContext, cells: &mut [&mut [CCell]]) {
        let width = min(
            cells.get(0).map(|c| c.len()).unwrap_or(0),
            self.cells.get(0).map(|c| c.len()).unwrap_or(0),
        );
        let height = min(cells.len(), self.cells.len());

        for x in 0..width {
            for y in 0..height {
                cells[y][x] = self.cells[y][x].clone().into();
            }
        }
    }

    fn cursor(&self, ctx: &UIContext) -> Option<Cursor> {
        self.cursor
    }
}

impl Drawable for Statusline {
    fn draw(&self, ctx: &UIContext, cells: &mut [&mut [CCell]]) {
        let style = ctx.style(ThemeField::Statusline);
        let width = cells.get(0).map(|c| c.len()).unwrap_or(0);
        for (i, cell) in into_cells_with_theme_pad_with(&self.line, &style, width)
            .into_iter()
            .enumerate()
        {
            cells[0][i] = cell;
        }
    }

    fn cursor(&self, ctx: &UIContext) -> Option<Cursor> {
        None
    }
}

impl Drawable for CustomPrompt {
    fn draw(&self, ctx: &UIContext, mut cells: &mut [&mut [CCell]]) {
        let wsize = size(cells);
        let default_style = ctx.theme.get(ThemeField::PromptDefault);
        let input_style = ctx.theme.get(ThemeField::PromptUserInput);

        match self.style {
            PromptStyle::Oneline => {
                let message_style = ctx.theme.get(ThemeField::PromptMessage);
                let mut message = into_cells_with_style(&self.prompt.message, message_style);
                let colon = into_cells_with_style(": ", message_style);
                message.extend(colon);

                let input = into_cells_with_style(&self.prompt.input, input_style);
                message.extend(input);
                pad_line(&mut message, default_style, wsize.width);
                put_line(message, 0, cells);

                cells = &mut cells[1..];
                let wsize = size(cells);
                let max_opts = wsize.height;
                self.prompt
                    .options
                    .iter()
                    .take(max_opts)
                    .enumerate()
                    .for_each(|(i, opt)| {
                        let field = if Some(i) == self.prompt.selected {
                            ThemeField::PromptCompletionSelected
                        } else {
                            ThemeField::PromptCompletion
                        };
                        let style = ctx.style(field);
                        put_line(
                            into_cells_with_style_pad(&opt.name, style, wsize.width),
                            i,
                            cells,
                        );
                    });
            }
            PromptStyle::Overlay => {
                const TITLE_HEIGHT: usize = 2;
                if wsize.height > TITLE_HEIGHT {
                    // Title
                    let title_style = ctx.theme.get(ThemeField::PromptOlayTitle);
                    let title = into_cells_with_style(&self.prompt.message, title_style);
                    let title = center_pad(title, title_style, wsize.width);
                    put_line(title, 0, cells);

                    // Empty line
                    // let mut line = vec![];
                    // pad_line(&mut line, input_style, wsize.width);
                    // put_line(line, 1, cells);

                    // Message
                    let message_style = ctx.theme.get(ThemeField::PromptOlayMessage);
                    let mut message = into_cells_with_style(" > ", message_style);
                    let input = into_cells_with_style(&self.prompt.input, input_style);
                    message.extend(input);
                    pad_line(&mut message, message_style, wsize.width);
                    put_line(message, 1, cells);
                }

                cells = &mut cells[TITLE_HEIGHT..];

                // Options
                let pcompl = ctx.theme.get(ThemeField::PromptCompletion);
                set_style(cells, pcompl);
                cells = draw_border(Border::Margin, pcompl, cells);
                let wsize = size(cells);
                let max_opts = wsize.height;

                self.prompt
                    .options
                    .iter()
                    .take(max_opts)
                    .enumerate()
                    .for_each(|(i, opt)| {
                        let (field, dfield) = if Some(i) == self.prompt.selected {
                            (
                                ThemeField::PromptCompletionSelected,
                                ThemeField::PromptCompletionSelectedDescription,
                            )
                        } else {
                            (
                                ThemeField::PromptCompletion,
                                ThemeField::PromptCompletionDescription,
                            )
                        };
                        let style = ctx.style(field);
                        let dstyle = ctx.style(dfield);

                        put_line(
                            format_option(&opt.name, &opt.description, style, dstyle, wsize.width),
                            i,
                            cells,
                        );
                    });
            }
        }
    }

    fn cursor(&self, ctx: &UIContext) -> Option<Cursor> {
        match self.style {
            PromptStyle::Oneline => {
                let cursor_col = {
                    let input_cells_before_cursor =
                        self.prompt.input[..self.prompt.cursor].into_cells().len();
                    let msg = self.prompt.message.chars().count();
                    let extra = 2; // ": "
                    msg + extra + input_cells_before_cursor
                };
                let style = ctx.theme.get(ThemeField::Default);
                Some(Cursor {
                    bg: style.fg,
                    fg: style.bg,
                    point: Point {
                        x: cursor_col,
                        y: 0,
                    },
                    shape: CursorShape::Line(true),
                })
            }
            PromptStyle::Overlay => {
                let cursor_col = {
                    let input_cells_before_cursor =
                        self.prompt.input[..self.prompt.cursor].into_cells().len();
                    let extra = 3; // " > "
                    extra + input_cells_before_cursor
                };
                let style = ctx.theme.get(ThemeField::Default);
                Some(Cursor {
                    bg: style.fg,
                    fg: style.bg,
                    point: Point {
                        x: cursor_col,
                        y: 1,
                    },
                    shape: CursorShape::Line(true),
                })
            }
        }
    }
}

impl Drawable for StatusMessage {
    fn draw(&self, ctx: &UIContext, cells: &mut [&mut [CCell]]) {
        let field = match self.severity {
            Severity::Info => ThemeField::Info,
            Severity::Warn => ThemeField::Warn,
            Severity::Error => ThemeField::Error,
        };
        let style = ctx.style(field);
        let width = cells.get(0).map(|c| c.len()).unwrap_or(0);
        for (i, cell) in into_cells_with_theme_pad_with(&self.message, &style, width)
            .into_iter()
            .enumerate()
        {
            cells[0][i] = cell;
        }
    }

    fn cursor(&self, ctx: &UIContext) -> Option<Cursor> {
        None
    }
}
