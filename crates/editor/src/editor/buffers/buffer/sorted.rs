use std::ops::{Deref, Index, Range};

use super::BufferRange;

#[derive(Debug, Clone)]
pub(crate) struct SortedBufferRanges(Vec<BufferRange>);

impl SortedBufferRanges {
    pub fn iter(&self) -> std::slice::Iter<Range<u64>> {
        self.0.iter()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Deref for SortedBufferRanges {
    type Target = [BufferRange];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Index<usize> for SortedBufferRanges {
    type Output = Range<u64>;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl From<Vec<Range<u64>>> for SortedBufferRanges {
    fn from(mut value: Vec<Range<u64>>) -> Self {
        value.sort_by(|a, b| a.start.cmp(&b.start));
        SortedBufferRanges(value)
    }
}

impl From<Range<u64>> for SortedBufferRanges {
    fn from(mut value: Range<u64>) -> Self {
        SortedBufferRanges(vec![value])
    }
}
