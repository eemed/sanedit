use std::ops::{Index, Range};

use super::BufferRange;

#[derive(Debug, Clone)]
pub(crate) struct SortedRanges(Vec<BufferRange>);

impl SortedRanges {
    pub fn iter(&self) -> std::slice::Iter<Range<usize>> {
        self.0.iter()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Index<usize> for SortedRanges {
    type Output = Range<usize>;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl From<Vec<Range<usize>>> for SortedRanges {
    fn from(mut value: Vec<Range<usize>>) -> Self {
        value.sort_by(|a, b| a.start.cmp(&b.start));
        SortedRanges(value)
    }
}
