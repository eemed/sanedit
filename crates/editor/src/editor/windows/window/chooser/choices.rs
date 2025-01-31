use std::cell::RefCell;

use sanedit_utils::sorted_vec::SortedVec;

use crate::common::matcher::ScoredChoice;

#[derive(Debug, Default)]
struct MergedOptions {
    /// Currently processed options, cursor for each sorted vec
    cursors: Vec<usize>,
    /// Indices to Options->options
    merged: Vec<(usize, usize)>,
}

impl MergedOptions {
    pub fn clear(&mut self, n: usize) {
        self.merged.clear();

        for i in 0..self.cursors.len() {
            self.cursors[i] = 0;
        }

        while self.cursors.len() < n {
            self.cursors.push(0);
        }
    }
}

#[derive(Debug, Default)]
pub struct Choices {
    options: Vec<SortedVec<ScoredChoice>>,
    total: usize,
    interior: RefCell<MergedOptions>,
}

impl Choices {
    pub fn push(&mut self, opts: SortedVec<ScoredChoice>) {
        self.total += opts.len();
        self.options.push(opts);

        let len = self.options.len();
        let mut interior = self.interior.borrow_mut();
        interior.clear(len);
    }

    pub fn len(&self) -> usize {
        self.total
    }

    pub fn is_empty(&self) -> bool {
        self.total == 0
    }

    pub fn get(&self, idx: usize) -> Option<&ScoredChoice> {
        self.merge_until(idx);
        let interior = self.interior.borrow();
        let (list, pos) = interior.merged.get(idx)?;
        let item = &self.options[*list][*pos];
        Some(item)
    }

    /// Create one sorted list by merging the locally sorted lists
    ///
    /// To avoid always merging when options are requested.
    /// The work done is saved using interior mutability
    fn merge_until(&self, idx: usize) {
        let mut interior = self.interior.borrow_mut();
        for _ in interior.merged.len()..idx + 1 {
            let len = interior.cursors.len();
            // min element match and the list/cursor index
            let mut min: Option<(usize, &ScoredChoice)> = None;

            for c in 0..len {
                let list = &self.options[c];
                let cursor = interior.cursors[c];
                if cursor >= list.len() {
                    continue;
                }
                let mat = &list[cursor];
                let replace = min
                    .as_ref()
                    .map(|(_, small_match)| mat < small_match)
                    .unwrap_or(true);

                if replace {
                    min = Some((c, mat));
                }
            }

            if let Some((idx, _)) = min {
                let pos = (idx, interior.cursors[idx]);
                interior.merged.push(pos);
                interior.cursors[idx] += 1;
            }
        }
    }
}
