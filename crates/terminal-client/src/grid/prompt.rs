use std::cmp::min;

use sanedit_messages::redraw::{
    choice::Range,
    prompt::{Prompt, Source},
    Cell, Cursor, CursorShape, IntoCells, Point, Style, ThemeField,
};

use crate::{grid::border::Border, ui::UIContext};

use super::{
    drawable::{DrawCursor, Drawable, Subgrid},
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

    pub fn rect(&mut self, screen: Rect) -> Rect {
        use PromptStyle::*;
        use Source::*;
        // Try to fit overlay prompt
        // magic number: overlay paddings 3 + prompt 1 + options + extra space so we
        // dont attach to window sides 6
        //
        // minimum height to draw overlay
        let Rect { width, height, .. } = screen;
        let olay_min_height = self.prompt.max_completions + 3 + 1 + 6;
        // height the overlay needs
        let olay_height = self.prompt.max_completions + 3 + 1;
        let oneline_min_height = self.prompt.max_completions + 1;
        self.style = match self.prompt.source {
            Search | Simple => Oneline,
            Prompt => {
                if height < olay_min_height {
                    Oneline
                } else {
                    Overlay
                }
            }
        };

        match self.style {
            PromptStyle::Oneline => Rect::new(0, 0, width, min(height, oneline_min_height)),
            PromptStyle::Overlay => {
                let rect_width = (width as f64 * 0.7) as usize;
                let x = (width - rect_width) / 2;
                let extra = height - olay_height;
                let y = extra / 4;
                Rect::new(x, y, rect_width, olay_height)
            }
        }
    }
}

impl Drawable for CustomPrompt {
    fn draw(&self, ctx: &UIContext, mut grid: Subgrid) {
        let wsize = grid.size();
        let width = wsize.width;

        match self.style {
            PromptStyle::Oneline => {
                let default_style = ctx.theme.get(ThemeField::PromptDefault);
                let input_style = ctx.theme.get(ThemeField::PromptUserInput);
                let message_style = ctx.theme.get(ThemeField::PromptMessage);
                let y = 0;
                let mut x = 0;

                for ch in self.prompt.message.chars() {
                    if x >= width {
                        break;
                    }

                    grid.replace(y, x, Cell::new_char(ch, message_style));
                    x += 1;
                }

                if x < width {
                    grid.replace(y, x, Cell::new_char(':', message_style));
                    x += 1;
                }
                if x < width {
                    grid.replace(y, x, Cell::new_char(' ', message_style));
                    x += 1;
                }

                for ch in self.prompt.input.chars() {
                    if x >= width {
                        break;
                    }

                    grid.replace(y, x, Cell::new_char(ch, input_style));
                    x += 1;
                }

                while x < width {
                    grid.replace(y, x, Cell::with_style(default_style));
                    x += 1;
                }

                let max_opts = wsize.height.saturating_sub(1);
                for (i, opt) in self.prompt.options.iter().take(max_opts).enumerate() {
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

                    let opts = ColumnFormatterOptions {
                        x: 0,
                        line: i + 1,
                        style,
                        match_style: mstyle,
                        description_style: dstyle,
                        width: wsize.width,
                    };

                    format_two_columns(&mut grid, &opt.text, &opt.description, &opt.matches, opts);
                }
            }
            PromptStyle::Overlay => {
                let input_style = ctx.theme.get(ThemeField::PromptOverlayInput);

                const TITLE_HEIGHT: usize = 2;
                if wsize.height > TITLE_HEIGHT {
                    // Title
                    let title_style = ctx.theme.get(ThemeField::PromptOverlayTitle);
                    let mut x = 0;
                    let mlen = self.prompt.message.chars().count();
                    let pad = (width.saturating_sub(mlen)) / 2;
                    for i in 0..pad {
                        grid.replace(0, i, Cell::with_style(title_style));
                        x += 1;
                    }

                    x += grid.put_string(0, x, &self.prompt.message, title_style);

                    while x < width {
                        grid.replace(0, x, Cell::with_style(title_style));
                        x += 1;
                    }

                    // Message
                    x = 0;
                    let message_style = ctx.theme.get(ThemeField::PromptOverlayMessage);

                    x += grid.put_string(1, x, " > ", message_style);
                    x += grid.put_string(1, x, &self.prompt.input, input_style);

                    while x < width {
                        grid.replace(1, x, Cell::with_style(message_style));
                        x += 1;
                    }
                }

                let mut rect = *grid.rect();
                rect.y += TITLE_HEIGHT;
                rect.height = rect.height.saturating_sub(2);
                let mut grid = grid.subgrid(&rect);

                // Borders
                let pcompl = ctx.theme.get(ThemeField::PromptCompletion);
                grid.clear_all(pcompl);
                let inside = grid.draw_border(Border::Margin, pcompl);
                if self.prompt.is_loading && &inside != grid.rect() && grid.width() >= 4 {
                    grid.replace(0, grid.width() - 2, Cell::new_char('.', pcompl));
                    grid.replace(0, grid.width() - 3, Cell::new_char('.', pcompl));
                }
                let mut grid = grid.subgrid(&inside);
                let wsize = grid.size();
                let max_opts = wsize.height;

                for (i, opt) in self.prompt.options.iter().take(max_opts).enumerate() {
                    if i >= wsize.height {
                        break;
                    }
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
                    let opts = ColumnFormatterOptions {
                        x: 0,
                        line: i,
                        style,
                        match_style: mstyle,
                        description_style: dstyle,
                        width: wsize.width,
                    };
                    format_two_columns(&mut grid, &opt.text, &opt.description, &opt.matches, opts);
                }
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

pub fn format_option(
    grid: &mut Subgrid<'_, '_>,
    item: &str,
    item_hls: &[Range<usize>],
    opts: &ColumnFormatterOptions,
) -> usize {
    let ColumnFormatterOptions {
        style,
        match_style,
        width,
        line,
        x: ox,
        ..
    } = opts;
    let mut x = *ox;
    let start = item.chars().count().saturating_sub(*width);

    for ch in item
        .chars()
        .skip(start)
        .map(|c| if c.is_control() { ' ' } else { c })
    {
        grid.replace(*line, x, Cell::new_char(ch, *style));
        x += 1;
    }

    for hl in item_hls {
        let mut pos = start;

        for i in *ox..x {
            let cell = grid.at(*line, i);
            if hl.contains(&pos) {
                cell.style = *match_style;
            }
            pos += cell.text.len()
        }
    }

    if start != 0 && *width > 2 {
        grid.at(*line, *ox).text = ".".into();
        grid.at(*line, *ox + 1).text = ".".into();
    }

    x - ox
}

pub(crate) struct ColumnFormatterOptions {
    line: usize,
    x: usize,
    style: Style,
    match_style: Style,
    description_style: Style,
    width: usize,
}

pub(crate) fn format_two_columns(
    grid: &mut Subgrid<'_, '_>,
    item: &str,
    description: &str,
    item_matches: &[Range<usize>],
    mut opts: ColumnFormatterOptions,
) -> usize {
    let ColumnFormatterOptions {
        style,
        description_style,
        width,
        line,
        x: ox,
        ..
    } = opts;
    // Pad first and last char as ' '
    let mut x = 0;
    grid.replace(line, x, Cell::with_style(style));
    x += 1;

    opts.width = opts.width.saturating_sub(2);
    opts.x = x;
    let ilen = format_option(grid, item, item_matches, &opts);
    x += ilen;

    if x < width {
        grid.replace(line, x, Cell::with_style(style));
        x += 1;
    }

    let dlen = description.chars().count();
    while x + dlen < width - 1 {
        grid.replace(line, x, Cell::with_style(style));
        x += 1;
    }

    for ch in description
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
    {
        if x >= width {
            break;
        }

        grid.replace(line, x, Cell::new_char(ch, description_style));
        x += 1;
    }

    while x < width {
        grid.replace(line, x, Cell::with_style(style));
        x += 1;
    }

    x - ox
}
