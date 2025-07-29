use serde::{Deserialize, Serialize};
use std::ops::{Add, RangeBounds, Sub};

pub type BufferRange = Range<u64>;

/// Normal range is shit
/// Start is inclusive, end is exclusive
#[derive(Default, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Hash)]
pub struct Range<T: Ord> {
    pub start: T,
    pub end: T,
}

impl<T: Ord> Range<T> {
    pub fn new(start: T, end: T) -> Self {
        Range { start, end }
    }

    pub fn overlaps(&self, other: &Self) -> bool {
        self.start < other.end && other.start < self.end
    }

    pub fn includes(&self, other: &Self) -> bool {
        self.start <= other.start && other.end <= self.end
    }

    pub fn contains(&self, other: &T) -> bool {
        &self.start <= other && other < &self.end
    }
}

impl<T: Ord + Sub + Clone + Copy> Range<T> {
    pub fn len(&self) -> <T as Sub<T>>::Output {
        self.end - self.start
    }

    pub fn is_empty(&self) -> bool {
        self.end == self.start
    }
}

impl<T: Ord + Add<Output = T> + Clone + Copy> Range<T> {
    pub fn forward(&mut self, off: T) {
        self.start = self.start + off;
        self.end = self.end + off;
    }
}

impl<T: Ord + Sub<Output = T> + Clone + Copy> Range<T> {
    pub fn backward(&mut self, off: T) {
        self.start = self.start - off;
        self.end = self.end - off;
    }
}

impl<T: Ord + Clone> From<Range<T>> for std::ops::Range<T> {
    fn from(value: Range<T>) -> Self {
        value.start..value.end
    }
}

impl<T: Ord + Clone> From<&Range<T>> for std::ops::Range<T> {
    fn from(value: &Range<T>) -> Self {
        value.start.clone()..value.end.clone()
    }
}

impl<T: Ord> RangeBounds<T> for &Range<T> {
    fn start_bound(&self) -> std::ops::Bound<&T> {
        std::ops::Bound::Included(&self.start)
    }

    fn end_bound(&self) -> std::ops::Bound<&T> {
        std::ops::Bound::Excluded(&self.end)
    }
}

impl<T: Ord> RangeBounds<T> for Range<T> {
    fn start_bound(&self) -> std::ops::Bound<&T> {
        std::ops::Bound::Included(&self.start)
    }

    fn end_bound(&self) -> std::ops::Bound<&T> {
        std::ops::Bound::Excluded(&self.end)
    }
}

impl<T: Ord> From<std::ops::Range<T>> for Range<T> {
    fn from(value: std::ops::Range<T>) -> Self {
        Range {
            start: value.start,
            end: value.end,
        }
    }
}
