use std::ops::Range;

/// A range in the buffer
pub type BufferRange = Range<u64>;

pub trait BufferRangeExt {
    fn len(&self) -> u64;
}

impl BufferRangeExt for BufferRange {
    fn len(&self) -> u64 {
        self.end - self.start
    }
}

pub trait RangeUtils {
    /// Wether this and other range overlap
    fn overlaps(&self, other: &Self) -> bool;

    /// Wether this range includes the other range
    fn includes(&self, other: &Self) -> bool;
}

impl<T: PartialOrd> RangeUtils for Range<T> {
    fn overlaps(&self, other: &Self) -> bool {
        self.start < other.end && other.start < self.end
    }

    fn includes(&self, other: &Self) -> bool {
        self.start <= other.start && other.end <= self.end
    }
}
