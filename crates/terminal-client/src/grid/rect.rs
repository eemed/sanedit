use std::cmp::min;

use sanedit_messages::redraw::{Point, Size};

use super::ccell::CCell;

#[derive(Clone, Debug, Copy)]
pub(crate) struct Rect {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
}

impl Rect {
    pub fn new(x: usize, y: usize, width: usize, height: usize) -> Rect {
        Rect {
            x,
            y,
            width,
            height,
        }
    }

    pub fn top_center(width: usize, height: usize) -> Rect {
        let width = width / 2;
        let height = height / 2;
        let x = width / 2;
        let y = height / 4;

        Rect {
            x,
            y,
            width,
            height,
        }
    }

    pub fn centered(width: usize, height: usize) -> Rect {
        let width = width / 2;
        let height = height / 2;
        let x = width / 2;
        let y = height / 2;

        Rect {
            x,
            y,
            width,
            height,
        }
    }

    pub fn position(&self) -> Point {
        Point {
            x: self.x,
            y: self.y,
        }
    }

    pub fn size(&self) -> Size {
        Size {
            width: self.width,
            height: self.height,
        }
    }

    pub fn grid(&self) -> Vec<Vec<CCell>> {
        vec![vec![CCell::transparent(); self.width]; self.height]
    }

    pub fn split_off(&mut self, split: Split) -> Rect {
        match split {
            Split::Top(split) => {
                let amount = split.get(self.height);
                self.y += amount;
                self.height -= amount;

                Rect {
                    x: self.x,
                    y: self.y - amount,
                    width: self.width,
                    height: amount,
                }
            }
            Split::Bottom(split) => {
                let amount = split.get(self.height);
                self.height -= amount;

                Rect {
                    x: self.x,
                    y: self.y + self.height,
                    width: self.width,
                    height: amount,
                }
            }
            Split::Left(split) => {
                let amount = split.get(self.width);
                self.x += amount;
                self.width -= amount;

                Rect {
                    x: self.x - amount,
                    y: self.y,
                    width: amount,
                    height: self.height,
                }
            }
            Split::Right(split) => {
                let amount = split.get(self.width);
                self.width -= amount;

                Rect {
                    x: self.x + self.width,
                    y: self.y,
                    width: amount,
                    height: self.height,
                }
            }
        }
    }
}

pub(crate) enum SplitPoint {
    Percentage(usize),
    Size(usize),
}

impl SplitPoint {
    pub fn get(&self, size: usize) -> usize {
        match self {
            SplitPoint::Percentage(p) => (size * p) / 100,
            SplitPoint::Size(s) => min(*s, size),
        }
    }
}

pub(crate) enum Split {
    Top(SplitPoint),
    Bottom(SplitPoint),
    Left(SplitPoint),
    Right(SplitPoint),
}

impl Split {
    pub fn top_size(size: usize) -> Split {
        Split::Top(SplitPoint::Size(size))
    }

    pub fn bottom_size(size: usize) -> Split {
        Split::Bottom(SplitPoint::Size(size))
    }

    pub fn left_size(size: usize) -> Split {
        Split::Left(SplitPoint::Size(size))
    }

    pub fn right_size(size: usize) -> Split {
        Split::Right(SplitPoint::Size(size))
    }
}
