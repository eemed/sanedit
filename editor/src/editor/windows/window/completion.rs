use std::cmp::{self, Ordering};

#[derive(Debug, Default)]
pub(crate) struct Completion {
    pub(crate) options: Vec<String>,
    pub(crate) selected: Option<usize>,
    pub(crate) must_complete: bool,
}

impl Completion {
    pub fn new(must_complete: bool) -> Completion {
        Completion {
            options: vec![],
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

    pub fn sort_options(&mut self, input: &str) {
        fn matches_with(string: &str, input: &str) -> Option<usize> {
            string.to_lowercase().find(input)
        }

        self.options.sort_by(|a, b| {
            let a_match = matches_with(a, input);
            let b_match = matches_with(b, input);

            match (a_match, b_match) {
                (None, None) => Ordering::Equal,
                (None, Some(_bn)) => Ordering::Less,
                (Some(_an), None) => Ordering::Greater,
                (Some(an), Some(bn)) => an.partial_cmp(&bn).unwrap(),
            }
        });
    }

    pub fn options(&self) -> &Vec<String> {
        &self.options
    }

    pub fn selected(&self) -> Option<(usize, &str)> {
        let sel = self.selected?;
        let opt = self.options.get(sel)?;
        Some((sel, opt.as_str()))
    }
}
