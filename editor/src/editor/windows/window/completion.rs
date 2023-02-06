use std::cmp;

#[derive(Debug, Default)]
pub(crate) struct Completion {
    pub(crate) options: Vec<String>,
    pub(crate) matches: Vec<usize>,
    pub(crate) selected: Option<usize>,
    pub(crate) must_complete: bool,
}

impl Completion {
    pub fn new(must_complete: bool) -> Completion {
        Completion {
            options: vec![],
            matches: vec![],
            selected: None,
            must_complete,
        }
    }

    pub fn select_next(&mut self) {
        if self.options.is_empty() {
            return;
        }
        if let Some(sel) = self.selected.as_mut() {
            *sel = cmp::min(*sel, self.options.len().saturating_sub(1));
        }
    }

    pub fn select_prev(&mut self) {
        if self.options.is_empty() {
            return;
        }
        if let Some(sel) = self.selected.as_mut() {
            *sel = sel.saturating_sub(1);
        }
    }

    pub fn provide_options(&mut self, options: Vec<String>) {
        self.options.extend(options);
    }

    pub fn match_options(&mut self, input: &str) {
        fn matches_with(string: &str, input: &str) -> Option<usize> {
            string.to_lowercase().find(input)
        }

        self.matches = self
            .options
            .iter()
            .enumerate()
            .filter(|(_, opt)| matches_with(opt, input).is_some())
            .map(|(i, _)| i)
            .collect();
    }

    pub fn matches(&self) -> Vec<&str> {
        self.matches
            .iter()
            .map(|pos| self.options[*pos].as_str())
            .collect()
    }

    pub fn all_options(&self) -> &Vec<String> {
        &self.options
    }

    pub fn selected(&self) -> Option<(usize, &str)> {
        let sel = self.selected?;
        let opt = self.options.get(sel)?;
        Some((sel, opt.as_str()))
    }
}
