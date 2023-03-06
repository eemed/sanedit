use std::ops::Index;

use super::inst::InstIndex;

pub(crate) type Thread = InstIndex;

pub(crate) struct ThreadSet {
    threads: SparseSet,
}

impl ThreadSet {
    pub fn with_capacity(cap: usize) -> ThreadSet {
        ThreadSet {
            threads: SparseSet::with_capacity(cap),
        }
    }

    pub fn add_thread(&mut self, thread: Thread) {
        self.threads.insert(thread);
    }

    pub fn clear(&mut self) {
        self.threads.clear();
    }

    pub fn len(&self) -> usize {
        self.threads.len()
    }
}

impl Index<usize> for ThreadSet {
    type Output = usize;

    fn index(&self, index: usize) -> &Self::Output {
        &self.threads[index]
    }
}

struct SparseSet {
    /// Holds the indices
    sparse: Box<[usize]>,

    /// Holds the elements
    dense: Vec<usize>,
}

impl SparseSet {
    pub fn with_capacity(cap: usize) -> SparseSet {
        SparseSet {
            sparse: vec![0; cap].into(),
            dense: Vec::with_capacity(cap),
        }
    }

    pub fn insert(&mut self, item: usize) {
        if self.contains(item) {
            return;
        }

        let n = self.dense.len();
        self.dense.push(item);
        self.sparse[item] = n;
    }

    pub fn contains(&self, item: usize) -> bool {
        let n = self.sparse[item];
        self.dense.get(n) == Some(&item)
    }

    pub fn clear(&mut self) {
        self.dense.clear();
    }

    pub fn len(&self) -> usize {
        self.dense.len()
    }
}

impl Index<usize> for SparseSet {
    type Output = usize;

    fn index(&self, index: usize) -> &Self::Output {
        &self.dense[index]
    }
}
