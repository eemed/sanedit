use std::cmp::min;

use sanedit_messages::redraw::{Point, Size};

use super::ccell::CCell;

#[derive(Clone, Debug, Copy, Default)]
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

    /// Whether this rect can include other rect
    pub fn includes(&self, other: &Rect) -> bool {
        self.x <= other.x
            && self.y <= other.y
            && self.x + self.width >= other.x + other.width
            && self.y + self.height >= other.y + other.height
    }

    pub fn contains(&self, point: &Point) -> bool {
        self.x <= point.x
            && point.x < self.x + self.width
            && self.y <= point.y
            && point.y < self.y + self.height
    }

    /// Rightmost position this rect reaches x + width
    pub fn rightmost(&self) -> usize {
        self.x + self.width
    }
}

#[allow(dead_code)]
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

#[allow(dead_code)]
pub(crate) enum Split {
    Top(SplitPoint),
    Bottom(SplitPoint),
    Left(SplitPoint),
    Right(SplitPoint),
}

#[allow(dead_code)]
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
