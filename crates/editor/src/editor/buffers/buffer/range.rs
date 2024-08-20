use std::ops::Range;

/// A range in the buffer
pub(crate) type BufferRange = Range<u64>;

pub(crate) trait BufferRangeExt {
    fn len(&self) -> u64;
}

impl BufferRangeExt for BufferRange {
    fn len(&self) -> u64 {
        self.end - self.start
    }
}
