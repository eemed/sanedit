use sanedit_messages::redraw::{Point, Popup, Severity, ThemeField};

use crate::ui::UIContext;

use super::{
    border::{draw_side_border, Border},
    ccell::{clear_all, into_cells_with_style, size, CCell},
    drawable::{DrawCursor, Drawable},
    item::GridItem,
    Rect,
};

pub(crate) fn open_popup(screen: Rect, win: Rect, popup: Popup) -> GridItem<Popup> {
    let over = above(&screen, &win, &popup);
    let screen_point = popup.point + win.position();
    if !over.contains(&screen_point) {
        return GridItem::new(popup, over);
    }

    let under = below(&screen, &win, &popup);
    if under.contains(&screen_point) {
        GridItem::new(popup, over)
    } else {
        GridItem::new(popup, under)
    }
}

pub(crate) fn below(screen: &Rect, win: &Rect, popup: &Popup) -> Rect {
    let Point { x, mut y } = popup.point + win.position();
    let width = popup
        .messages
        .iter()
        .filter_map(|msg| msg.text.lines().map(|line| line.len() + 2).max())
        .max()
        .unwrap_or(0)
        .min(screen.width);
    let height = (popup
        .messages
        .iter()
        .map(|msg| msg.text.lines().count())
        .sum::<usize>()
        + popup.messages.len().saturating_sub(1))
    .min(screen.height);

    if y + height < screen.height {
        y += 1;
    }

    Rect {
        x,
        y,
        width,
        height,
    }
}

pub(crate) fn above(screen: &Rect, win: &Rect, popup: &Popup) -> Rect {
    let Point { mut x, mut y } = popup.point + win.position();
    let width = popup
        .messages
        .iter()
        .filter_map(|msg| msg.text.lines().map(|line| line.len() + 2).max())
        .max()
        .unwrap_or(0)
        .min(screen.width);
    let height = (popup
        .messages
        .iter()
        .map(|msg| msg.text.lines().count())
        .sum::<usize>()
        + popup.messages.len().saturating_sub(1))
    .min(screen.height);
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
        cells = draw_side_border(Border::Margin, style, cells);
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
                    Some(Severity::Hint) => ThemeField::Hint,
                    Some(Severity::Info) => ThemeField::Info,
                    Some(Severity::Warn) => ThemeField::Warn,
                    Some(Severity::Error) => ThemeField::Error,
                    None => ThemeField::PopupDefault,
                };
                ctx.style(field)
            };
            // Add popup messages
            for line in msg.text.lines().skip(self.line_offset) {
                let lcells = into_cells_with_style(line, mstyle);
                for cell in lcells {
                    if col == wsize.width {
                        row += 1;
                        col = 0;

                        if row == wsize.height {
                            break;
                        }
                    }

                    cells[row][col] = cell;
                    col += 1;
                }

                // Line processed goto next
                row += 1;
                col = 0;

                if row == wsize.height {
                    break;
                }
            }
        }
    }

    fn cursor(&self, _ctx: &UIContext) -> DrawCursor {
        DrawCursor::Hide
    }
}
