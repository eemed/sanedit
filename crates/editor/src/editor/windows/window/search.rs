use sanedit_core::{BufferRange, SearchDirection, SearchKind};

#[derive(Debug, Default)]
pub(crate) struct LastSearch {
    /// The searched pattern
    pub pattern: String,

    pub kind: SearchKind,
    pub dir: SearchDirection,

    /// The match if the last search matched something
    pub current_match: Option<BufferRange>,
}

#[derive(Debug)]
pub(crate) struct Search {
    last_search: Option<LastSearch>,

    pub direction: SearchDirection,
    pub kind: SearchKind,

    /// Search matches that should be highlighted
    pub hl_matches: Vec<BufferRange>,
    /// Whether to highlight last match or not
    pub hl_last: bool,

    /// Current search match
    pub current_match: Option<BufferRange>,
}

impl Search {
    pub fn save_last_search(&mut self, pattern: &str) {
        self.last_search = Some(LastSearch {
            pattern: pattern.into(),
            kind: self.kind,
            dir: self.direction,
            current_match: self.current_match.clone(),
        });

        self.current_match = None;
    }

    pub fn last_search(&self) -> Option<&LastSearch> {
        self.last_search.as_ref()
    }

    pub fn last_search_pattern(&self) -> Option<&str> {
        self.last_search.as_ref().map(|ls| ls.pattern.as_str())
    }
}

impl Default for Search {
    fn default() -> Self {
        Search {
            last_search: None,
            direction: SearchDirection::Forward,
            kind: SearchKind::default(),
            hl_last: false,
            hl_matches: vec![],
            current_match: None,
        }
    }
}
