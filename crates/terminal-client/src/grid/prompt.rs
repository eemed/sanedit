use std::cmp::min;

use sanedit_messages::redraw::{
    Cursor, CursorShape, IntoCells, Point, Prompt, Source, Style, ThemeField,
};

use crate::{
    grid::{
        border::{draw_border, Border},
        ccell::{center_pad, set_style},
    },
    ui::UIContext,
};

use super::{
    ccell::{into_cells_with_style, into_cells_with_style_pad, pad_line, put_line, size, CCell},
    drawable::{DrawCursor, Drawable},
    item::GridItem,
    Rect,
};

#[derive(Debug, Clone, Copy)]
pub enum PromptStyle {
    /// Simple one line prompt with options on another lines
    Oneline,
    /// An overlay window
    Overlay,
}

#[derive(Debug)]
pub struct CustomPrompt {
    pub style: PromptStyle,
    pub prompt: Prompt,
}

pub(crate) fn open_prompt(width: usize, height: usize, prompt: Prompt) -> GridItem<CustomPrompt> {
    use PromptStyle::*;
    use Source::*;
    // Try to fit overlay prompt
    // magic number: overlay paddings 3 + prompt 1 + options + extra space so we
    // dont attach to window sides 6
    //
    // minimum height to draw overlay
    let olay_min_height = prompt.max_completions + 3 + 1 + 6;
    // height the overlay needs
    let olay_height = prompt.max_completions + 3 + 1;
    let oneline_min_height = prompt.max_completions + 1;
    let style = match prompt.source {
        Search | Simple => Oneline,
        Prompt => {
            if height < olay_min_height {
                Oneline
            } else {
                Overlay
            }
        }
    };

    match style {
        PromptStyle::Oneline => {
            let rect = Rect::new(0, 0, width, min(height, oneline_min_height));
            GridItem::new(CustomPrompt { prompt, style }, rect)
        }
        PromptStyle::Overlay => {
            let width = width / 2;
            let x = width / 2;
            let extra = height - olay_height;
            let y = extra / 4;
            let rect = Rect::new(x, y, width, olay_height);
            GridItem::new(CustomPrompt { prompt, style }, rect)
        }
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
                        let mstyle = ctx.style(field);
                        let mut line = into_cells_with_style_pad(&opt.name, style, wsize.width);

                        // Highlight matches
                        for mat in &opt.matches {
                            let mut pos = 0;
                            for cell in &mut line {
                                if mat.contains(&pos) {
                                    cell.style = mstyle;
                                }
                                pos += cell.cell.text.len();
                            }
                        }

                        put_line(line, i, cells);
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
                        let (field, dfield, mfield) = if Some(i) == self.prompt.selected {
                            (
                                ThemeField::PromptCompletionSelected,
                                ThemeField::PromptCompletionSelectedDescription,
                                ThemeField::PromptCompletionSelectedMatch,
                            )
                        } else {
                            (
                                ThemeField::PromptCompletion,
                                ThemeField::PromptCompletionDescription,
                                ThemeField::PromptCompletionMatch,
                            )
                        };
                        let style = ctx.style(field);
                        let dstyle = ctx.style(dfield);
                        let mstyle = ctx.style(mfield);

                        let mut line =
                            format_option(&opt.name, &opt.description, style, dstyle, wsize.width);

                        // Highlight matches
                        for mat in &opt.matches {
                            let mut pos = 0;
                            // dont count padding
                            for cell in &mut line[1..] {
                                if mat.contains(&pos) {
                                    cell.style = mstyle;
                                }
                                pos += cell.cell.text.len();
                            }
                        }

                        put_line(line, i, cells);
                    });
            }
        }
    }

    fn cursor(&self, ctx: &UIContext) -> DrawCursor {
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
                DrawCursor::Show(Cursor {
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
                DrawCursor::Show(Cursor {
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

pub(crate) fn format_option(
    left: &str,
    right: &str,
    mstyle: Style,
    dstyle: Style,
    width: usize,
) -> Vec<CCell> {
    let mut left = {
        let mut res = String::from(" ");
        res.push_str(&left);
        res.push(' ');
        res
    };

    let right = {
        let mut res = String::from("");
        res.push_str(&right);
        res.push(' ');
        res
    };

    // Fill space between
    let mut len = left.len() + right.len();
    while len < width {
        left.push(' ');
        len += 1;
    }

    let mut result = into_cells_with_style(&left, mstyle);
    result.extend(into_cells_with_style(&right, dstyle));
    result.truncate(width);
    result
}
