use serde::{Deserialize, Serialize};
use std::ops::{Add, RangeBounds, Sub};

pub type BufferRange = Range<u64>;

/// Normal range is shit for u64
/// Cannot implement rangebounds for &Range<u64>
/// A just works range type with copy
/// Start is inclusive, end is exclusive
#[derive(
    Default, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Hash,
)]
pub struct Range<T: Ord> {
    pub start: T,
    pub end: T,
}

impl<T: Ord> Range<T> {}

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

impl Range<u64> {
    pub fn from_bounds<R: RangeBounds<u64>>(bounds: R) -> Range<u64> {
        let start = match bounds.start_bound() {
            std::ops::Bound::Included(n) => *n,
            std::ops::Bound::Excluded(n) => n.saturating_sub(1),
            std::ops::Bound::Unbounded => 0,
        };
        let end = match bounds.end_bound() {
            std::ops::Bound::Included(n) => *n + 1,
            std::ops::Bound::Excluded(n) => *n,
            std::ops::Bound::Unbounded => u64::MAX,
        };

        Range { start, end }
    }

    pub fn overlaps<R: RangeBounds<u64>>(&self, other: R) -> bool {
        let other = Range::<u64>::from_bounds(other);
        self.start < other.end && other.start < self.end
    }

    pub fn includes<R: RangeBounds<u64>>(&self, other: R) -> bool {
        let other = Range::<u64>::from_bounds(other);
        self.start <= other.start && other.end <= self.end
    }

    pub fn contains(&self, other: &u64) -> bool {
        &self.start <= other && other < &self.end
    }
}

impl Range<usize> {
    pub fn from_bounds<R: RangeBounds<usize>>(bounds: R) -> Range<usize> {
        let start = match bounds.start_bound() {
            std::ops::Bound::Included(n) => *n,
            std::ops::Bound::Excluded(n) => n.saturating_sub(1),
            std::ops::Bound::Unbounded => 0,
        };
        let end = match bounds.end_bound() {
            std::ops::Bound::Included(n) => *n + 1,
            std::ops::Bound::Excluded(n) => *n,
            std::ops::Bound::Unbounded => usize::MAX,
        };

        Range { start, end }
    }

    pub fn overlaps<R: RangeBounds<usize>>(&self, other: R) -> bool {
        let other = Range::<usize>::from_bounds(other);
        self.start < other.end && other.start < self.end
    }

    pub fn includes<R: RangeBounds<usize>>(&self, other: R) -> bool {
        let other = Range::<usize>::from_bounds(other);
        self.start <= other.start && other.end <= self.end
    }

    pub fn contains(&self, other: &usize) -> bool {
        &self.start <= other && other < &self.end
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
