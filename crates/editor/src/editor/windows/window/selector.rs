mod options;

pub(crate) use options::*;
use sanedit_utils::sorted_vec::SortedVec;

/// Selects one item from a list of options.
/// Options can be filtered down using an input string.
#[derive(Debug, Default)]
pub(crate) struct Selector {
    pub(crate) options: Options,

    /// Currently selected index from `options`
    pub(crate) selected: Option<usize>,
}

impl Selector {
    pub fn new() -> Selector {
        Selector {
            options: Options::default(),
            selected: None,
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

    pub fn provide_options(&mut self, opts: SortedVec<SelectorOption>) {
        self.options.push(opts)
    }

    pub fn selected_pos(&self) -> Option<usize> {
        self.selected
    }

    pub fn selected(&self) -> Option<(usize, &str)> {
        let sel = self.selected?;
        let opt = self.options.get(sel)?;
        Some((sel, opt.value()))
    }

    /// Returns less than or equal to count matches around selection,
    /// selection is positioned at the selected_offset index.
    pub fn matches_window(&self, count: usize, offset: usize) -> Vec<&SelectorOption> {
        let mut results = Vec::with_capacity(count);
        for i in offset..offset + count {
            if let Some(item) = self.options.get(i) {
                results.push(item);
            }
        }

        results
    }
}
