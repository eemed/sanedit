use sanedit_messages::redraw::{Point, Popup, Severity, Size, ThemeField};

use crate::ui::UIContext;

use super::{
    border::{draw_border, Border},
    ccell::{clear_all, into_cells_with_style, size, CCell},
    drawable::{DrawCursor, Drawable},
    Rect,
};

pub(crate) fn popup_rect(screen: Rect, win: Rect, popup: &Popup) -> Rect {
    let over = above(&screen, &win, &popup);
    let screen_point = popup.point + win.position();
    if !over.contains(&screen_point) {
        return over;
    }

    let under = below(&screen, &win, &popup);
    if under.contains(&screen_point) {
        over
    } else {
        under
    }
}

// How much to reserve for border
const BORDER: Border = Border::Margin;
const BORDER_VERTICAL: usize = 2;
const BORDER_HORIZONTAL: usize = 2;

fn popup_size(screen: &Rect, popup: &Popup) -> Size {
    let width = popup
        .messages
        .iter()
        .filter_map(|msg| {
            msg.text
                .lines()
                .map(|line| line.len() + BORDER_HORIZONTAL)
                .max()
        })
        .max()
        .unwrap_or(0)
        .min(screen.width);
    let height = (popup
        .messages
        .iter()
        .map(|msg| {
            msg.text
                .lines()
                .map(|line| (line.len()).div_ceil(width - BORDER_HORIZONTAL).max(1))
                .sum::<usize>()
        })
        .sum::<usize>()
        + BORDER_VERTICAL
        + popup.messages.len().saturating_sub(1))
    .min(screen.height);
    Size { width, height }
}

fn below(screen: &Rect, win: &Rect, popup: &Popup) -> Rect {
    let Point { mut x, mut y } = popup.point + win.position();
    let Size { width, height } = popup_size(screen, popup);

    y += 1;

    if y + height > screen.height {
        y = 0;
    }

    if x + width >= screen.width {
        x -= x + width - screen.width;
    }

    Rect {
        x,
        y,
        width,
        height,
    }
}

fn above(screen: &Rect, win: &Rect, popup: &Popup) -> Rect {
    let Point { mut x, mut y } = popup.point + win.position();
    let Size { width, height } = popup_size(screen, popup);
    y = y.saturating_sub(height);

    if x + width >= screen.width {
        x -= x + width - screen.width;
    }

    Rect {
        x,
        y,
        width,
        height,
    }
}

impl Drawable for Popup {
    fn draw(&self, ctx: &UIContext, mut cells: &mut [&mut [CCell]]) {
        let style = ctx.style(ThemeField::PopupDefault);

        clear_all(cells, style);
        cells = draw_border(BORDER, style, cells);
        let wsize = size(cells);

        let mut row = 0;
        let mut col = 0;

        for (i, msg) in self.messages.iter().enumerate() {
            // Add popup message separators
            if i != 0 {
                let lcells = into_cells_with_style(&"─".repeat(wsize.width - 1), style);
                lcells.into_iter().enumerate().for_each(|(i, cell)| {
                    cells[row][i] = cell;
                });

                row += 1;

                if row == wsize.height {
                    break;
                }
            }

            let mstyle = {
                let field = match msg.severity {
                    Some(Severity::Hint) => ThemeField::PopupHint,
                    Some(Severity::Info) => ThemeField::PopupInfo,
                    Some(Severity::Warn) => ThemeField::PopupWarn,
                    Some(Severity::Error) => ThemeField::PopupError,
                    None => ThemeField::PopupDefault,
                };
                ctx.style(field)
            };
            // Add popup messages
            for line in msg.text.lines().skip(self.line_offset) {
                let lcells = into_cells_with_style(line, mstyle);
                for cell in lcells {
                    if col >= wsize.width {
                        row += 1;
                        col = 0;

                        if row >= wsize.height {
                            break;
                        }
                    }

                    cells[row][col] = cell;
                    col += 1;
                }

                // Line processed goto next
                row += 1;
                col = 0;

                if row >= wsize.height {
                    break;
                }
            }
        }
    }

    fn cursor(&self, ctx: &UIContext) -> DrawCursor {
        if ctx.rect.contains(&ctx.cursor_position) {
            DrawCursor::Hide
        } else {
            DrawCursor::Ignore
        }
    }
}
