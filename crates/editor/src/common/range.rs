use std::ops::Range;

pub(crate) trait RangeUtils {
    fn overlaps(&self, other: &Self) -> bool;
}

impl<T: PartialOrd> RangeUtils for Range<T> {
    fn overlaps(&self, other: &Self) -> bool {
        self.start <= other.end && other.start <= self.end
    }
}
