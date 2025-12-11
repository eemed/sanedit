mod choices;

use sanedit_utils::sorted_vec::SortedVec;

use crate::common::ScoredChoice;

pub use self::choices::Choices;

/// Selects one item from a list of options.
/// Options can be filtered down using an input string.
#[derive(Debug, Default)]
pub struct Chooser {
    choices: Choices,

    /// Currently selected index from `options`
    selected: Option<usize>,
}

impl Chooser {
    pub fn new() -> Chooser {
        Chooser {
            choices: Choices::default(),
            selected: None,
        }
    }

    pub fn options(&self) -> &Choices {
        &self.choices
    }

    pub fn select_next(&mut self) {
        if self.choices.is_empty() {
            self.selected = None;
        }

        match self.selected {
            Some(n) => {
                let is_last = n == self.choices.len() - 1;
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
        if self.choices.is_empty() {
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
            None => self.selected = Some(self.choices.len() - 1),
        }
    }

    pub fn add(&mut self, opts: SortedVec<ScoredChoice>) {
        self.choices.push(opts)
    }

    pub fn selected_pos(&self) -> Option<usize> {
        self.selected
    }

    pub fn selected(&self) -> Option<&ScoredChoice> {
        let sel = self.selected?;
        self.choices.get(sel)
    }

    pub fn len(&self) -> usize {
        self.choices.len()
    }

    /// Returns less than or equal to count matches around selection,
    /// selection is positioned at the selected_offset index.
    pub fn matches_window(&self, count: usize, offset: usize) -> Vec<&ScoredChoice> {
        let mut results = Vec::with_capacity(count);
        for i in offset..offset + count {
            if let Some(item) = self.choices.get(i) {
                results.push(item);
            }
        }

        results
    }
}
