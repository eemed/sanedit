use std::cmp::max;

use sanedit_messages::redraw::{Completion, Point, Size};

use super::{canvas::Canvas, Rect};

pub(crate) fn open_completion(win: Rect, compl: Completion) -> Canvas<Completion> {
    let below = below(win, &compl);
    if below.fits_inside(&win) {
        return Canvas::new(compl, below);
    }

    let above = above(win, &compl);
    if above.fits_inside(&win) {
        return Canvas::new(compl, above);
    }

    // TODO shrink
    Canvas::new(compl, below)
}

fn below(win: Rect, compl: &Completion) -> Rect {
    let Point { mut x, y } = compl.point + win.position() + Point { x: 0, y: 1 };
    x = x.saturating_sub(compl.query_len);
    let Size { width, height } = compl.preferred_size();
    Rect {
        x,
        y,
        width,
        height,
    }
}

fn above(win: Rect, compl: &Completion) -> Rect {
    let Point { mut x, mut y } = compl.point + win.position();
    let width = compl.options.iter().fold(0, |acc, o| {
        max(
            acc,
            o.name.chars().count() + 1 + o.description.chars().count(),
        )
    });
    let height = compl.options.len();
    y -= compl.options.len();
    x = x.saturating_sub(compl.query_len);

    Rect {
        x,
        y,
        width,
        height,
    }
}
