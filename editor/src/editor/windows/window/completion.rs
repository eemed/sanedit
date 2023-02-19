use std::{cmp, mem};

#[derive(Debug, Default)]
pub(crate) struct Completion {
    /// All completion options
    pub(crate) options: Vec<String>,
    /// Currently matched completions. Completions are juggled between this and
    /// `all` depending on wether they are matched or not
    pub(crate) matched: Vec<String>,

    /// Currently selected completion index from `matched`
    pub(crate) selected: Option<usize>,

    // Wether the completion must be one of the candidates
    pub(crate) must_complete: bool,

    pub smartcase: bool,
}

impl Completion {
    pub fn new(must_complete: bool) -> Completion {
        Completion {
            options: vec![],
            matched: vec![],
            selected: None,
            must_complete,
            smartcase: true,
        }
    }

    pub fn select_next(&mut self) {
        if self.matched.is_empty() {
            return;
        }

        if let Some(sel) = self.selected.as_mut() {
            *sel = cmp::min(self.matched.len().saturating_sub(1), *sel + 1);
        } else {
            self.selected = Some(0);
        }
    }

    pub fn select_prev(&mut self) {
        if self.matched.is_empty() {
            return;
        }
        if let Some(sel) = self.selected.as_mut() {
            *sel = sel.saturating_sub(1);
        } else {
            let last = self.matched.len().saturating_sub(1);
            self.selected = Some(last);
        }
    }

    pub fn provide_options(&mut self, options: Vec<String>) {
        self.options.extend(options);
    }

    pub fn match_options(&mut self, input: &str) {
        fn matches_with(string: &str, input: &str, ignore_case: bool) -> Option<usize> {
            if ignore_case {
                string.to_ascii_lowercase().find(input)
            } else {
                string.find(input)
            }
        }

        let mut opts = mem::take(&mut self.matched);
        opts.extend(mem::take(&mut self.options));

        let ignore_case = self.smartcase
            && input.chars().fold(true, |acc, ch| {
                acc && (!ch.is_ascii_alphabetic() || ch.is_ascii_lowercase())
            });

        for opt in opts.into_iter() {
            if let Some(_) = matches_with(&opt, &input, ignore_case) {
                let len = opt.len();
                let ins_idx = self
                    .matched
                    .binary_search_by(|res| {
                        use std::cmp::Ordering;
                        match res.len().cmp(&len) {
                            Ordering::Less => Ordering::Less,
                            Ordering::Greater => Ordering::Greater,
                            Ordering::Equal => res.cmp(&opt),
                        }
                    })
                    .unwrap_or_else(|x| x);
                self.matched.insert(ins_idx, opt);
            } else {
                self.options.push(opt);
            }
        }
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
