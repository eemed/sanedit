use std::cmp::min;

use sanedit_messages::redraw::{
    Completion, Cursor, CursorShape, IntoCells, Item, ItemKind, Point, Popup, Severity, Size,
    StatusMessage, Statusline, Style, ThemeField, Window,
};

use crate::{grid::ccell::format_option, ui::UIContext};

use super::{
    border::{draw_border, Border},
    ccell::{
        center_pad, clear_all, format_completion, into_cells_with_style, into_cells_with_style_pad,
        into_cells_with_theme_pad_with, pad_line, put_line, set_style, size, CCell,
    },
    items::{CustomItems, Kind},
    prompt::{CustomPrompt, PromptStyle},
};

#[derive(Debug)]
pub enum DrawCursor {
    /// Hide cursor from view
    Hide,
    /// Show cursor
    Show(Cursor),
    /// Keep where ever the cursor is now
    Ignore,
}

pub(crate) trait Drawable {
    fn draw(&self, ctx: &UIContext, cells: &mut [&mut [CCell]]);
    fn cursor(&self, ctx: &UIContext) -> DrawCursor;
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

    fn cursor(&self, ctx: &UIContext) -> DrawCursor {
        match self.cursor {
            Some(cursor) => DrawCursor::Show(cursor),
            None => DrawCursor::Ignore,
        }
    }
}

impl Drawable for Statusline {
    fn draw(&self, ctx: &UIContext, cells: &mut [&mut [CCell]]) {
        let style = ctx.style(ThemeField::Statusline);
        let width = cells.get(0).map(|c| c.len()).unwrap_or(0);
        let left = into_cells_with_theme_pad_with(&self.left, &style, width);
        for (i, cell) in left.into_iter().enumerate() {
            cells[0][i] = cell;
        }

        let right = into_cells_with_style(&self.right, style);
        for (i, cell) in right.into_iter().rev().enumerate() {
            let pos = width - 1 - i;
            let c = &mut cells[0][pos];
            if c.is_blank() {
                *c = cell;
            } else {
                break;
            }
        }
    }

    fn cursor(&self, ctx: &UIContext) -> DrawCursor {
        DrawCursor::Ignore
    }
}

impl Drawable for Completion {
    fn draw(&self, ctx: &UIContext, cells: &mut [&mut [CCell]]) {
        let wsize = size(cells);
        let max_opts = wsize.height;
        self.options
            .iter()
            .take(max_opts)
            .enumerate()
            .for_each(|(i, opt)| {
                let (field, dfield) = if Some(i) == self.selected {
                    (
                        ThemeField::CompletionSelected,
                        ThemeField::CompletionSelectedDescription,
                    )
                } else {
                    (ThemeField::Completion, ThemeField::CompletionDescription)
                };
                let style = ctx.style(field);
                let dstyle = ctx.style(dfield);

                let line = format_completion(
                    &opt.name,
                    &opt.description,
                    style,
                    dstyle,
                    wsize.width,
                    ctx.rect.x != 0,
                );

                put_line(line, i, cells);
            });
    }

    fn cursor(&self, ctx: &UIContext) -> DrawCursor {
        DrawCursor::Ignore
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

    fn cursor(&self, ctx: &UIContext) -> DrawCursor {
        DrawCursor::Ignore
    }
}

impl CustomItems {
    fn draw_filetree(&self, ctx: &UIContext, cells: &mut [&mut [CCell]]) {
        let fill = ctx.style(ThemeField::FiletreeDefault);
        let file = ctx.style(ThemeField::FiletreeFile);
        let dir = ctx.style(ThemeField::FiletreeDir);
        let markers = ctx.style(ThemeField::FiletreeMarkers);
        let sel = ctx.style(ThemeField::FiletreeSelected);

        clear_all(cells, fill);

        let Size { width, height } = size(cells);
        let last = width.saturating_sub(1);

        for i in 0..height {
            let mut ccell = CCell::from('│');
            ccell.style = markers;
            cells[i][last] = ccell;
        }

        for i in 0..height {
            let line = std::mem::replace(&mut cells[i], &mut []);
            cells[i] = &mut line[..last];
        }

        let Size { width, height } = size(cells);
        for (row, item) in self.items.items.iter().skip(self.scroll).enumerate() {
            if row >= cells.len() {
                break;
            }

            let style = if self.scroll + row == self.items.selected {
                sel
            } else {
                match item.kind {
                    ItemKind::Group { expanded } => dir,
                    ItemKind::Item => file,
                }
            };

            let mut titem = Self::format_ft_item(item, style, markers);
            pad_line(&mut titem, fill, width);

            for (i, cell) in titem.into_iter().enumerate() {
                cells[row][i] = cell;
            }
        }
    }

    fn format_ft_item(item: &Item, name: Style, extra: Style) -> Vec<CCell> {
        let mut result = vec![];
        result.extend(into_cells_with_style(&"  ".repeat(item.level), extra));

        match item.kind {
            ItemKind::Group { expanded } => {
                if expanded {
                    result.extend(into_cells_with_style("- ", extra));
                } else {
                    result.extend(into_cells_with_style("+ ", extra));
                }
            }
            ItemKind::Item => {
                result.extend(into_cells_with_style("# ", extra));
            }
        }

        result.extend(into_cells_with_style(&item.name, name));

        if matches!(item.kind, ItemKind::Group { .. }) {
            result.extend(into_cells_with_style("/", name));
        }

        result
    }

    fn draw_locations(&self, ctx: &UIContext, mut cells: &mut [&mut [CCell]]) {
        let fill = ctx.style(ThemeField::LocationsDefault);
        let entry = ctx.style(ThemeField::LocationsEntry);
        let group = ctx.style(ThemeField::LocationsGroup);
        let markers = ctx.style(ThemeField::LocationsMarkers);
        let smarkers = ctx.style(ThemeField::LocationsSelectedMarkers);
        let title = ctx.style(ThemeField::LocationsTitle);
        let sel = ctx.style(ThemeField::LocationsSelected);
        let lmat = ctx.style(ThemeField::LocationsMatch);
        let smat = ctx.style(ThemeField::LocationsSelectedMatch);

        clear_all(cells, fill);

        let Size { width, height } = size(cells);
        if !cells.is_empty() {
            let mut line = into_cells_with_style(" Locations", title);

            for _ in line.len()..cells[0].len() {
                let mut ccell = CCell::from(' ');
                ccell.style = title;
                line.push(ccell);
            }

            line.truncate(width);

            for (i, c) in line.into_iter().enumerate() {
                cells[0][i] = c;
            }

            cells = &mut cells[1..];
        }

        for (row, item) in self.items.items.iter().skip(self.scroll).enumerate() {
            if row >= cells.len() {
                break;
            }

            let width = cells.get(0).map(|c| c.len()).unwrap_or(0);
            let is_selected = self.scroll + row == self.items.selected;
            let style = if is_selected {
                sel
            } else {
                match item.kind {
                    ItemKind::Group { expanded } => group,
                    ItemKind::Item => entry,
                }
            };
            let mat = if is_selected { smat } else { lmat };
            let fil = if is_selected { sel } else { fill };
            let mark = if is_selected { smarkers } else { markers };
            let mut titem = Self::format_loc_item(item, style, mark, mat, fil);
            pad_line(&mut titem, fil, width);

            for (i, cell) in titem.into_iter().enumerate() {
                cells[row][i] = cell;
            }
        }
    }

    fn format_loc_item(
        item: &Item,
        name: Style,
        extra: Style,
        mat: Style,
        fill: Style,
    ) -> Vec<CCell> {
        let mut result = vec![];
        result.extend(into_cells_with_style(&"  ".repeat(item.level), fill));

        match item.kind {
            ItemKind::Group { expanded } => {
                if expanded {
                    result.extend(into_cells_with_style("- ", extra));
                } else {
                    result.extend(into_cells_with_style("+ ", extra));
                }
            }
            ItemKind::Item => {
                let line = item.line.map(|l| l.to_string()).unwrap_or("?".into());
                result.extend(into_cells_with_style(&format!("{line}: "), extra));
            }
        }

        let mut name = into_cells_with_style(&item.name, name);

        // Highlight matches
        for hl in &item.highlights {
            let mut pos = 0;
            // dont count padding
            for cell in &mut name {
                if hl.contains(&pos) {
                    cell.style = mat;
                }
                pos += cell.cell.text.len();
            }
        }

        result.extend(name);
        result
    }
}

impl Drawable for CustomItems {
    fn draw(&self, ctx: &UIContext, cells: &mut [&mut [CCell]]) {
        match self.kind {
            Kind::Filetree => self.draw_filetree(ctx, cells),
            Kind::Locations => self.draw_locations(ctx, cells),
        }
    }

    fn cursor(&self, ctx: &UIContext) -> DrawCursor {
        if self.items.in_focus {
            DrawCursor::Hide
        } else {
            DrawCursor::Ignore
        }
    }
}

impl Drawable for Popup {
    fn draw(&self, ctx: &UIContext, cells: &mut [&mut [CCell]]) {
        todo!()
    }

    fn cursor(&self, ctx: &UIContext) -> DrawCursor {
        DrawCursor::Ignore
    }
}
