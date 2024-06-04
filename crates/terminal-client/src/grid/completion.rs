use std::cmp::{max, min};

use sanedit_messages::redraw::{Completion, Point, Size};

use super::{item::GridItem, Rect};

const MIN_WIDTH: usize = 40;
const MIN_HEIGHT: usize = 5;

pub(crate) fn open_completion(win: Rect, compl: Completion) -> GridItem<Completion> {
    let mut below = below(win, &compl);
    if win.includes(&below) {
        return GridItem::new(compl, below);
    }

    let above = above(win, &compl);
    if win.includes(&above) {
        return GridItem::new(compl, above);
    }

    let fb = fallback(win, &compl);

    GridItem::new(compl, fb)
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
    x = x.saturating_sub(compl.query_len + 1);
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

    y = y.saturating_sub(compl.options.len());
    x = x.saturating_sub(compl.query_len + 1);

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
