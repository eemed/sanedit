use std::cmp::max;

use sanedit_messages::redraw::{Completion, Point, Size};

use super::{canvas::Canvas, Rect};

pub(crate) fn open_completion(win: Rect, compl: Completion) -> Canvas<Completion> {
    let rect = below(win, &compl);
    Canvas::new(compl, rect)
}

fn below(win: Rect, compl: &Completion) -> Rect {
    let Point { x, y } = compl.point + win.position() + Point { x: 0, y: 1 };
    let Size { width, height } = compl.preferred_size();
    Rect {
        x,
        y,
        width,
        height,
    }
}

fn above(win: Rect, compl: &Completion) -> Rect {
    let Point { x, mut y } = compl.point + win.position();
    let width = compl
        .options
        .iter()
        .fold(0, |acc, o| max(acc, o.chars().count()));
    let height = compl.options.len();
    y -= compl.options.len();

    Rect {
        x,
        y,
        width,
        height,
    }
}
