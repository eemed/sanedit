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
    pub(crate) longest_item_text: usize,
    pub(crate) completion: Completion,
}

impl CustomCompletion {
    pub fn new(completion: Completion) -> CustomCompletion {
        let longest_item_text = completion
            .choices
            .iter()
            .map(|item| item.text.chars().count())
            .max()
            .unwrap_or(0);

        CustomCompletion {
            longest_item_text,
            completion,
        }
    }

    pub fn update(&mut self, diff: completion::Difference) {
        self.completion.update(diff);

        let longest_item_text = self
            .completion
            .choices
            .iter()
            .map(|item| item.text.chars().count())
            .max()
            .unwrap_or(0);

        self.longest_item_text = std::cmp::max(self.longest_item_text, longest_item_text);
    }

    pub fn rect(&self, win: Rect) -> Rect {
        let below = below(win, &self);
        if win.includes(&below) {
            return below;
        }

        let above = above(win, &self);
        if win.includes(&above) {
            return above;
        }

        fallback(win, &self)
    }
}

const MIN_WIDTH: usize = 40;
const MIN_HEIGHT: usize = 5;

/// Size of completion where everything fits on screen
fn preferred_size(compl: &CustomCompletion) -> Size {
    // [pad] [left_column] [pad] [right_column] [pad]
    let width = compl
        .completion
        .choices
        .iter()
        .map(|o| {
            let mut len = 1 + compl.longest_item_text + 1;

            if !o.description.is_empty() {
                len += o.description.chars().count();
                len += 1;
            }

            len
        })
        .max()
        .unwrap_or(0);
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

fn above(win: Rect, compl: &CustomCompletion) -> Rect {
    let Point { mut x, mut y } = compl.completion.point + win.position();
    let Size {
        mut width,
        mut height,
    } = preferred_size(compl);

    y = y.saturating_sub(compl.completion.choices.len());
    x = x.saturating_sub(compl.completion.item_offset_before_point + 1);

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
                    self.longest_item_text,
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
    longest_item_text: usize,
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
        let pad_to = longest_item_text + if left_pad { 1 } else { 0 };
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
