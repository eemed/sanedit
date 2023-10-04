use std::ops::Index;

/// A vector that is always sorted
#[derive(Default, Debug, Clone)]
pub struct SortedVec<T: Ord> {
    items: Vec<T>,
}

impl<'a, T: Ord + Clone> SortedVec<T> {
    pub fn from_unsorted(items: &'a [T]) -> SortedVec<T> {
        let mut items = items.to_vec();
        items.sort();
        SortedVec { items }
    }
}

impl<'a, T: Ord> SortedVec<T> {
    pub fn new() -> SortedVec<T> {
        SortedVec { items: Vec::new() }
    }

    pub fn get(&self, i: usize) -> Option<&T> {
        self.items.get(i)
    }

    pub fn iter(&self) -> std::slice::Iter<T> {
        self.items.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn merge(&mut self, other: SortedVec<T>) {
        todo!()
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

impl<T: Ord> Index<usize> for SortedVec<T> {
    type Output = T;

    fn index(&self, i: usize) -> &Self::Output {
        &self.items[i]
    }
}

impl<T: Ord + Clone> From<&[T]> for SortedVec<T> {
    fn from(arr: &[T]) -> Self {
        Self::from_unsorted(arr)
    }
}

impl<T: Ord> From<Vec<T>> for SortedVec<T> {
    fn from(mut items: Vec<T>) -> Self {
        items.sort();
        SortedVec { items }
    }
}

impl<T: Ord> From<T> for SortedVec<T> {
    fn from(value: T) -> Self {
        SortedVec { items: vec![value] }
    }
}
