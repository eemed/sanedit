use std::cmp::{max, min};

use sanedit_messages::redraw::{completion::Completion, Point, Size, Style, ThemeField};

use crate::ui::UIContext;

use super::{
    ccell::{into_cells_with_style, put_line, size, CCell},
    drawable::{DrawCursor, Drawable},
    Rect,
};

const MIN_WIDTH: usize = 40;
const MIN_HEIGHT: usize = 5;

pub(crate) fn completion_rect(win: Rect, compl: &Completion) -> Rect {
    let below = below(win, compl);
    if win.includes(&below) {
        return below;
    }

    let above = above(win, compl);
    if win.includes(&above) {
        return above;
    }

    fallback(win, compl)
}

fn fallback(win: Rect, compl: &Completion) -> Rect {
    let mut below = below(win, compl);
    let Size { width, height } = compl.preferred_size();
    let minw = min(width, win.width);
    if below.width < minw {
        below.width = minw;
    }

    if below.rightmost() > win.rightmost() {
        below.x -= below.rightmost() - win.rightmost();
    }

    below.height = min(height, win.height - win.y);

    below
}

fn below(win: Rect, compl: &Completion) -> Rect {
    let Point { mut x, y } = compl.point + win.position() + Point { x: 0, y: 1 };
    x = x.saturating_sub(compl.item_offset_before_point + 1);
    let Size {
        mut width,
        mut height,
    } = compl.preferred_size();

    if x + width > win.x + win.width {
        width = max(win.width, MIN_WIDTH);
    }

    if y + height > win.y + win.height {
        height = max(win.height, MIN_HEIGHT);
    }

    Rect {
        x,
        y,
        width,
        height,
    }
}

fn above(win: Rect, compl: &Completion) -> Rect {
    let Point { mut x, mut y } = compl.point + win.position();
    let Size {
        mut width,
        mut height,
    } = compl.preferred_size();

    y = y.saturating_sub(compl.choices.len());
    x = x.saturating_sub(compl.item_offset_before_point + 1);

    if x + width > win.x + win.width {
        width = max(win.width, MIN_WIDTH);
    }

    if y + height > win.y + win.height {
        height = max(win.height, MIN_HEIGHT);
    }

    Rect {
        x,
        y,
        width,
        height,
    }
}

impl Drawable for Completion {
    fn draw(&self, ctx: &UIContext, cells: &mut [&mut [CCell]]) {
        let wsize = size(cells);
        let max_opts = wsize.height;
        self.choices
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
                    &opt.text,
                    &opt.description,
                    style,
                    dstyle,
                    wsize.width,
                    ctx.rect.x != 0,
                );

                put_line(line, i, cells);
            });
    }

    fn cursor(&self, _ctx: &UIContext) -> DrawCursor {
        DrawCursor::Ignore
    }
}

pub(crate) fn format_completion(
    left: &str,
    right: &str,
    mstyle: Style,
    dstyle: Style,
    width: usize,
    left_pad: bool,
) -> Vec<CCell> {
    let mut left = {
        let mut res = String::new();
        if left_pad {
            res.push(' ');
        }

        res.push_str(left);
        res.push(' ');
        res
    };

    let right = {
        let mut res = right.to_string();
        res.push(' ');
        res
    };

    // Fill space between
    let mut len = left.chars().count() + right.chars().count();
    while len < width {
        left.push(' ');
        len += 1;
    }

    let mut result = into_cells_with_style(&left, mstyle);
    result.extend(into_cells_with_style(&right, dstyle));
    result.truncate(width);
    result
}
