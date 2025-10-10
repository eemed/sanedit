use sanedit_messages::redraw::{Cell, Point, Popup, PopupMessageText, Severity, Size, ThemeField};

use crate::ui::UIContext;

use super::{
    border::Border,
    drawable::{DrawCursor, Drawable, Subgrid},
    Rect,
};

pub(crate) fn popup_rect(screen: Rect, win: Rect, popup: &Popup) -> Rect {
    let over = above(&screen, &win, popup);
    let screen_point = popup.point + win.position();
    if !over.contains(&screen_point) {
        return over;
    }

    let under = below(&screen, &win, popup);
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
        .filter_map(|msg| match &msg.text {
            PopupMessageText::Formatted(cells) => cells
                .iter()
                .map(|line| line.len() + BORDER_HORIZONTAL)
                .max(),
            PopupMessageText::Plain(text) => text
                .lines()
                .map(|line| line.chars().count() + BORDER_HORIZONTAL)
                .max(),
        })
        .max()
        .unwrap_or(0)
        .min(screen.width);
    let height = (popup
        .messages
        .iter()
        .map(|msg| match &msg.text {
            PopupMessageText::Formatted(cells) => cells
                .iter()
                .map(|line| (line.len()).div_ceil(width - BORDER_HORIZONTAL).max(1))
                .sum::<usize>(),
            PopupMessageText::Plain(text) => text
                .lines()
                .map(|line| {
                    (line.chars().count())
                        .div_ceil(width - BORDER_HORIZONTAL)
                        .max(1)
                })
                .sum::<usize>(),
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
    fn draw(&self, ctx: &UIContext, mut grid: Subgrid) {
        let style = ctx.style(ThemeField::PopupDefault);

        grid.clear_all(style);
        let inside = grid.draw_border(BORDER, style);
        let mut grid = grid.subgrid(&inside);
        let wsize = grid.size();

        let mut row = 0;
        let mut col = 0;

        for (i, msg) in self.messages.iter().enumerate() {
            if row == wsize.height {
                break;
            }

            // Add popup message separators
            if i != 0 {
                for i in 0..wsize.width {
                    grid.replace(row, i, Cell::new_char('─', style))
                }

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

            // log::info!("msg: {:?}", msg.text);
            // Add popup messages
            match &msg.text {
                PopupMessageText::Formatted(cells) => {
                    for line in cells.iter().skip(self.line_offset) {
                        for cell in line {
                            if col >= wsize.width {
                                row += 1;
                                col = 0;

                                if row >= wsize.height {
                                    break;
                                }
                            }

                            grid.replace(row, col, cell.clone());
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
                PopupMessageText::Plain(text) => {
                    for line in text.lines().skip(self.line_offset) {
                        for cell in line
                            .chars()
                            .map(|ch| if ch.is_control() { ' ' } else { ch })
                            .map(|ch| Cell::new_char(ch, mstyle))
                        {
                            if col >= wsize.width {
                                row += 1;
                                col = 0;

                                if row >= wsize.height {
                                    break;
                                }
                            }

                            grid.replace(row, col, cell);
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
