use std::ops::Index;

/// A vector that is always sorted
#[derive(Debug, Clone)]
pub enum SortedVec<'a, T: Ord> {
    Ref(&'a [T]),
    Owned(Vec<T>),
}

impl<'a, T: Ord + Clone> SortedVec<'a, T> {
    pub fn from_unsorted(positions: &'a [T]) -> SortedVec<'a, T> {
        if is_sorted(positions) {
            Self::Ref(positions)
        } else {
            let mut positions = positions.to_vec();
            positions.sort();
            Self::Owned(positions)
        }
    }
}

impl<'a, T: Ord> SortedVec<'a, T> {
    pub fn iter(&self) -> std::slice::Iter<T> {
        match self {
            SortedVec::Ref(poss) => poss.iter(),
            SortedVec::Owned(poss) => poss.iter(),
        }
    }

    pub fn merge<'b>(&mut self, other: SortedVec<'b, T>) {
        // // Merge the two arrays by comparing score
        // let cap = opts.len() + self.options.len();
        // let old = mem::replace(&mut self.options, Vec::with_capacity(cap));

        // let n = min(old.len(), opts.len());
        // let mut i = 0;
        // let mut j = 0;

        // while i < n && j < n {
        //     if old[i].score() < opts[j].score() {
        //         self.options.push(old[i].clone());
        //         i += 1;
        //     } else {
        //         self.options.push(opts[j].clone());
        //         j += 1;
        //     }
        // }

        // while i < old.len() {
        //     self.options.push(old[i].clone());
        //     i += 1;
        // }

        // while j < opts.len() {
        //     self.options.push(opts[j].clone());
        //     j += 1;
        // }
    }
}

impl<'a, T: Ord> Index<usize> for SortedVec<'a, T> {
    type Output = T;

    fn index(&self, i: usize) -> &Self::Output {
        match self {
            SortedVec::Ref(opts) => &opts[i],
            SortedVec::Owned(opts) => &opts[i],
        }
    }
}

impl<'a, T: Ord + Clone> From<&'a [T]> for SortedVec<'a, T> {
    fn from(arr: &'a [T]) -> Self {
        Self::from_unsorted(arr)
    }
}

impl<'a, T: Ord> From<Vec<T>> for SortedVec<'a, T> {
    fn from(mut arr: Vec<T>) -> Self {
        arr.sort();
        Self::Owned(arr)
    }
}

impl<'a, T: Ord> From<T> for SortedVec<'a, T> {
    fn from(value: T) -> Self {
        Self::Owned(vec![value])
    }
}

fn is_sorted<T: Ord>(arr: &[T]) -> bool {
    let mut min = None;

    for item in arr {
        if let Some(min) = min {
            if min > item {
                return false;
            }
        }

        min = Some(item);
    }

    true
}
