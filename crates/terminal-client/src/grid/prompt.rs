use std::cmp::min;

use sanedit_messages::redraw::{
    prompt::{Prompt, Source},
    Cursor, CursorShape, IntoCells, Point, Style, ThemeField,
};

use crate::{
    grid::{
        border::{draw_border_no_strip, strip_border, Border},
        ccell::{center_pad, set_style},
    },
    ui::UIContext,
};

use super::{
    ccell::{into_cells_with_style, into_cells_with_style_pad, pad_line, put_line, size, CCell},
    drawable::{DrawCursor, Drawable},
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

impl CustomPrompt {
    pub fn new(prompt: Prompt) -> CustomPrompt {
        CustomPrompt {
            style: PromptStyle::Oneline,
            prompt,
        }
    }
}

pub(crate) fn prompt_rect(screen: Rect, prompt: &mut CustomPrompt) -> Rect {
    use PromptStyle::*;
    use Source::*;
    // Try to fit overlay prompt
    // magic number: overlay paddings 3 + prompt 1 + options + extra space so we
    // dont attach to window sides 6
    //
    // minimum height to draw overlay
    let Rect { width, height, .. } = screen;
    let olay_min_height = prompt.prompt.max_completions + 3 + 1 + 6;
    // height the overlay needs
    let olay_height = prompt.prompt.max_completions + 3 + 1;
    let oneline_min_height = prompt.prompt.max_completions + 1;
    prompt.style = match prompt.prompt.source {
        Search | Simple => Oneline,
        Prompt => {
            if height < olay_min_height {
                Oneline
            } else {
                Overlay
            }
        }
    };

    match prompt.style {
        PromptStyle::Oneline => Rect::new(0, 0, width, min(height, oneline_min_height)),
        PromptStyle::Overlay => {
            let width = width / 2;
            let x = width / 2;
            let extra = height - olay_height;
            let y = extra / 4;
            Rect::new(x, y, width, olay_height)
        }
    }
}

impl Drawable for CustomPrompt {
    fn draw(&self, ctx: &UIContext, mut cells: &mut [&mut [CCell]]) {
        let wsize = size(cells);

        match self.style {
            PromptStyle::Oneline => {
                let default_style = ctx.theme.get(ThemeField::PromptDefault);
                let input_style = ctx.theme.get(ThemeField::PromptUserInput);
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
                        let mut line = into_cells_with_style_pad(&opt.text, style, wsize.width);

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
                let input_style = ctx.theme.get(ThemeField::PromptOverlayInput);

                const TITLE_HEIGHT: usize = 2;
                if wsize.height > TITLE_HEIGHT {
                    // Title
                    let title_style = ctx.theme.get(ThemeField::PromptOverlayTitle);
                    let title = into_cells_with_style(&self.prompt.message, title_style);
                    let title = center_pad(title, title_style, wsize.width);
                    put_line(title, 0, cells);

                    // Message
                    let message_style = ctx.theme.get(ThemeField::PromptOverlayMessage);
                    let mut message = into_cells_with_style(" > ", message_style);
                    let input = into_cells_with_style(&self.prompt.input, input_style);
                    message.extend(input);
                    pad_line(&mut message, message_style, wsize.width);
                    put_line(message, 1, cells);
                }

                cells = &mut cells[TITLE_HEIGHT..];

                // Borders
                let pcompl = ctx.theme.get(ThemeField::PromptCompletion);
                set_style(cells, pcompl);
                draw_border_no_strip(Border::Margin, pcompl, cells);

                // Ignore border cells from this point on
                cells = strip_border(cells);

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

                        let mut line = cells_left_right(
                            &opt.text,
                            &opt.description,
                            style,
                            dstyle,
                            wsize.width,
                        );

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

pub(crate) fn cells_left_right(
    left: &str,
    right: &str,
    mstyle: Style,
    dstyle: Style,
    width: usize,
) -> Vec<CCell> {
    let mut left = {
        let mut res = String::from(" ");
        res.push_str(left);
        res.push(' ');
        res
    };

    let right = {
        let mut res = String::from("");
        res.push_str(right);
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
