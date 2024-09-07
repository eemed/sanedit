use sanedit_messages::redraw::{Point, Popup, ThemeField};

use crate::ui::UIContext;

use super::{
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
    let Point { mut x, mut y } = popup.point + win.position();
    let width = popup
        .lines
        .iter()
        .map(|l| l.len())
        .max()
        .unwrap_or(0)
        .min(screen.width);
    let height = popup.lines.len().min(screen.height);

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
        .lines
        .iter()
        .map(|l| l.len())
        .max()
        .unwrap_or(0)
        .min(screen.width);
    let height = popup.lines.len().min(screen.height);
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
    fn draw(&self, ctx: &UIContext, cells: &mut [&mut [CCell]]) {
        let wsize = size(cells);
        let style = ctx.style(ThemeField::PopupDefault);

        clear_all(cells, style);

        let mut row = 0;
        let mut col = 0;

        for line in &self.lines {
            let lcells = into_cells_with_style(line.as_str(), style);
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

    fn cursor(&self, ctx: &UIContext) -> DrawCursor {
        DrawCursor::Hide
    }
}
