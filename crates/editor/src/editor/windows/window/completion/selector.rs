/// Selects one item from a list of options.
/// Options can be filtered down using an input string.
#[derive(Debug, Default)]
pub(crate) struct Selector {
    /// Currently matched completions. Completions are juggled between this and
    /// `all` depending on wether they are matched or not
    pub(crate) matched: Vec<String>,

    /// Currently selected index from `matched`
    pub(crate) selected: Option<usize>,

    pub smartcase: bool,
}

impl Selector {
    pub fn new() -> Selector {
        Selector {
            matched: vec![],
            selected: None,
            smartcase: true,
        }
    }

    pub fn select_next(&mut self) {
        if self.matched.is_empty() {
            self.selected = None;
        }

        match self.selected {
            Some(n) => {
                let is_last = n == self.matched.len() - 1;
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
        if self.matched.is_empty() {
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
            None => self.selected = Some(self.matched.len() - 1),
        }
    }

    pub fn provide_options(&mut self, options: Vec<String>) {
        self.matched.extend(options);
    }

    pub fn selected_pos(&self) -> Option<usize> {
        self.selected
    }

    pub fn selected(&self) -> Option<(usize, &str)> {
        let sel = self.selected?;
        let opt = self.matched.get(sel)?;
        Some((sel, opt.as_str()))
    }

    /// Returns less than or equal to count matches around selection,
    /// selection is positioned at the selected_offset index.
    pub fn matches_window(&self, count: usize, offset: usize) -> Vec<&str> {
        self.matched
            .iter()
            .skip(offset)
            .take(count)
            .map(|s| s.as_str())
            .collect()
    }
}
