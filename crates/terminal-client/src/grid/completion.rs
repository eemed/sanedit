use std::cmp::{max, min};

use sanedit_messages::redraw::{completion::Completion, Cell, Point, Size, Style, ThemeField};

use crate::ui::UIContext;

use super::{
    drawable::{DrawCursor, Drawable, Subgrid},
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

    pub fn update(&mut self, compl: Completion) {
        self.completion = compl;

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
        let below = below(win, self);
        if win.includes(&below) {
            return below;
        }

        let above = above(win, self);
        if win.includes(&above) {
            return above;
        }

        fallback(win, self)
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
        x = win.x + win.width.saturating_sub(width);
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
        x = win.x + win.width.saturating_sub(width);
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
    fn draw(&self, ctx: &UIContext, mut grid: Subgrid) {
        let wsize = grid.size();
        let max_opts = wsize.height;
        let left_pad = ctx.rect.x != 0;
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

                let opts = CompletionFormatOptions {
                    line: i,
                    x: 0,
                    mstyle: style,
                    dstyle,
                    width: wsize.width,
                    left_pad,
                    max_columns: &self.column_max_width,
                };
                format_completion(&mut grid, &opt.text, &opt.description, &opts);
            });
    }

    fn cursor(&self, _ctx: &UIContext) -> DrawCursor {
        DrawCursor::Ignore
    }
}

pub(crate) struct CompletionFormatOptions<'a> {
    line: usize,
    x: usize,
    mstyle: Style,
    dstyle: Style,
    width: usize,
    left_pad: bool,
    max_columns: &'a [usize],
}

pub(crate) fn format_completion<'a>(
    grid: &mut Subgrid<'_, '_>,
    left: &str,
    right: &str,
    opts: &CompletionFormatOptions<'a>,
) {
    let CompletionFormatOptions {
        mstyle,
        dstyle,
        width,
        left_pad,
        max_columns,
        x: ox,
        line,
    } = opts;
    let mut x = *ox;

    if *left_pad && x < *width {
        grid.replace(*line, x, Cell::with_style(*mstyle));
        x += 1;
    }

    for ch in left
        .chars()
        .map(|ch| if ch.is_control() { ' ' } else { ch })
    {
        if x >= *width {
            break;
        }

        grid.replace(*line, x, Cell::new_char(ch, *mstyle));
        x += 1;
    }

    let pad_to = max_columns[0] + if *left_pad { 1 } else { 0 };
    while x < pad_to && x < *width {
        grid.replace(*line, x, Cell::with_style(*mstyle));
        x += 1;
    }

    if x < *width {
        grid.replace(*line, x, Cell::with_style(*mstyle));
        x += 1;
    }

    for ch in right
        .chars()
        .map(|ch| if ch.is_control() { ' ' } else { ch })
    {
        if x >= *width {
            break;
        }

        grid.replace(*line, x, Cell::new_char(ch, *dstyle));
        x += 1;
    }

    while x < *width {
        grid.replace(*line, x, Cell::with_style(*mstyle));
        x += 1;
    }
}
