use std::cmp::{max, min};

use sanedit_messages::redraw::{
    completion::{self, Completion},
    Diffable, Point, Size, Style, ThemeField,
};

use crate::ui::UIContext;

use super::{
    ccell::{into_cells_with_style, put_line, size, CCell},
    drawable::{DrawCursor, Drawable},
    Rect,
};

pub(crate) struct CustomCompletion {
    pub(crate) column_max_width: Vec<usize>,
    pub(crate) completion: Completion,
}

impl CustomCompletion {
    pub fn new(completion: Completion) -> CustomCompletion {
        let first_col = completion
            .choices
            .iter()
            .map(|item| item.text.chars().count())
            .max()
            .unwrap_or(0);
        let second_col = completion
            .choices
            .iter()
            .map(|item| item.description.chars().count())
            .max()
            .unwrap_or(0);

        CustomCompletion {
            column_max_width: vec![first_col, second_col],
            completion,
        }
    }

    pub fn update(&mut self, diff: completion::Difference) {
        self.completion.update(diff);

        let first_col = self
            .completion
            .choices
            .iter()
            .map(|item| item.text.chars().count())
            .max()
            .unwrap_or(0);
        self.column_max_width[0] = max(self.column_max_width[0], first_col);

        let second_col = self
            .completion
            .choices
            .iter()
            .map(|item| item.description.chars().count())
            .max()
            .unwrap_or(0);
        self.column_max_width[1] = max(self.column_max_width[1], second_col);
    }

    pub fn rect(&self, win: Rect) -> Rect {
        let below = below(win, &self);
        if win.includes(&below) {
            log::info!("BELOW");
            return below;
        }

        let above = above(win, &self);
        if win.includes(&above) {
            log::info!("ABOVE");
            return above;
        }

        fallback(win, &self)
    }
}

/// Size of completion where everything fits on screen
fn preferred_size(compl: &CustomCompletion) -> Size {
    // [pad] [first_column] [pad] [second_column] [pad]
    let width = 1 + compl.column_max_width[0] + 1 + compl.column_max_width[1] + 1;
    let height = compl.completion.choices.len();
    Size { width, height }
}

fn fallback(win: Rect, compl: &CustomCompletion) -> Rect {
    let mut below = below(win, compl);
    let Size { width, height } = preferred_size(compl);
    let minw = min(width, win.width);
    if below.width < minw {
        below.width = minw;
    }

    if below.rightmost() > win.rightmost() {
        below.x -= below.rightmost() - win.rightmost();
    }


    below.height = min(height, win.height - win.y);

    if below.y + below.height > win.y + win.height {
        below.y = (win.y + win.height).saturating_sub(below.height + 1);
    }

    log::info!("FB: WIN: {win:?}, rect: {below:?}");
    below
}

fn below(win: Rect, compl: &CustomCompletion) -> Rect {
    let Point { mut x, y } = compl.completion.point + win.position() + Point { x: 0, y: 1 };
    x = x.saturating_sub(compl.completion.item_offset_before_point + 1);
    let Size {
        mut width,
        mut height,
    } = preferred_size(compl);

    if x + width > win.x + win.width {
        x = win.width - width;
    }

    if x + width > win.x + win.width {
        width = win.width;
    }

    if y + height > win.y + win.height {
        height = win.height;
    }

    Rect {
        x,
        y,
        width,
        height,
    }
}

fn above(win: Rect, compl: &CustomCompletion) -> Rect {
    let Point { mut x, mut y } = compl.completion.point + win.position();
    let Size {
        mut width,
        mut height,
    } = preferred_size(compl);

    y = y.saturating_sub(compl.completion.choices.len());
    x = x.saturating_sub(compl.completion.item_offset_before_point + 1);

    if x + width > win.x + win.width {
        x = win.width - width;
    }

    if x + width > win.x + win.width {
        width = win.width;
    }

    if y + height > win.y + win.height {
        height = win.height;
    }

    Rect {
        x,
        y,
        width,
        height,
    }
}

impl Drawable for CustomCompletion {
    fn draw(&self, ctx: &UIContext, cells: &mut [&mut [CCell]]) {
        let wsize = size(cells);
        let max_opts = wsize.height;
        let pad_left = ctx.rect.x != 0;
        self.completion
            .choices
            .iter()
            .take(max_opts)
            .enumerate()
            .for_each(|(i, opt)| {
                let (field, dfield) = if Some(i) == self.completion.selected {
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
                    pad_left,
                    &self.column_max_width,
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
    max_columns: &[usize],
) -> Vec<CCell> {
    let left = {
        let mut lleft = String::new();
        if left_pad {
            lleft.push(' ');
        }

        lleft.push_str(left);
        // Fill space between
        let n = lleft.chars().count();
        let mut i = 0;
        let pad_to = max_columns[0] + if left_pad { 1 } else { 0 };
        while i + n < pad_to {
            lleft.push(' ');
            i += 1;
        }

        lleft.push(' ');
        into_cells_with_style(&lleft, mstyle)
    };

    let right = {
        let mut right = right.to_string();
        let n = right.chars().count();
        let mut i = 0;
        while i + n < width {
            right.push(' ');
            i += 1;
        }

        into_cells_with_style(&right, dstyle)
    };

    let mut result = left;
    result.extend(right);
    result.truncate(width);

    result
}
