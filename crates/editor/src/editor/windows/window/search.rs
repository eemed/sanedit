use sanedit_core::{BufferRange, SearchKind};

#[derive(Debug, Default)]
pub(crate) struct LastSearch {
    /// The searched pattern
    pub pattern: String,

    pub kind: SearchKind,

    /// The match if the last search matched something
    pub current_match: Option<BufferRange>,
}

#[derive(Debug)]
pub(crate) struct Search {
    pub on_line_char_search: Option<char>,
    last_search: Option<LastSearch>,

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
            on_line_char_search: None,
            last_search: None,
            kind: SearchKind::default(),
            hl_last: false,
            hl_matches: vec![],
            current_match: None,
        }
    }
}
