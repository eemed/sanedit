use std::{cmp::min, mem};

use crate::common::matcher::Match;

/// Selects one item from a list of options.
/// Options can be filtered down using an input string.
#[derive(Debug, Default)]
pub(crate) struct Selector {
    /// all options
    pub(crate) options: Vec<Match>,

    /// Currently selected index from `options`
    pub(crate) selected: Option<usize>,

    pub smartcase: bool,
}

impl Selector {
    pub fn new() -> Selector {
        Selector {
            options: vec![],
            selected: None,
            smartcase: true,
        }
    }

    pub fn select_next(&mut self) {
        if self.options.is_empty() {
            self.selected = None;
        }

        match self.selected {
            Some(n) => {
                let is_last = n == self.options.len() - 1;
                if is_last {
                    self.selected = None;
                } else {
                    self.selected = Some(n + 1);
                }
            }
            None => self.selected = Some(0),
        }
    }

    pub fn select_prev(&mut self) {
        if self.options.is_empty() {
            return;
        }

        match self.selected {
            Some(n) => {
                let is_first = n == 0;
                if is_first {
                    self.selected = None;
                } else {
                    self.selected = Some(n - 1);
                }
            }
            None => self.selected = Some(self.options.len() - 1),
        }
    }

    // pub fn provide_options(&mut self, matches: Matches) {
    //     // Merge the two arrays by comparing score
    //     let cap = matches.len() + self.options.len();
    //     let old = mem::replace(&mut self.options, Vec::with_capacity(cap));

    //     let n = min(old.len(), matches.len());
    //     let mut i = 0;
    //     let mut j = 0;

    //     while i < n && j < n {
    //         if old[i].score() < matches[j].score() {
    //             self.options.push(old[i].clone());
    //             i += 1;
    //         } else {
    //             self.options.push(matches[j].clone());
    //             j += 1;
    //         }
    //     }

    //     while i < old.len() {
    //         self.options.push(old[i].clone());
    //         i += 1;
    //     }

    //     while j < matches.len() {
    //         self.options.push(matches[j].clone());
    //         j += 1;
    //     }
    // }

    pub fn selected_pos(&self) -> Option<usize> {
        self.selected
    }

    pub fn selected(&self) -> Option<(usize, &str)> {
        let sel = self.selected?;
        let opt = self.options.get(sel)?;
        Some((sel, opt.as_str()))
    }

    /// Returns less than or equal to count matches around selection,
    /// selection is positioned at the selected_offset index.
    pub fn matches_window(&self, count: usize, offset: usize) -> Vec<&str> {
        self.options
            .iter()
            .skip(offset)
            .take(count)
            .map(|s| s.as_str())
            .collect()
    }
}
