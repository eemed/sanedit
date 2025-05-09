use sanedit_core::{BufferRange, SearchOptions};

#[derive(Debug, Default)]
pub(crate) struct SearchResult {
    /// The searched pattern
    pub pattern: String,
    pub opts: SearchOptions,
    pub result: Option<BufferRange>,
}

#[derive(Debug)]
pub(crate) struct Search {
    // Used for on line searches
    pub on_line_char_search: Option<char>,
    pub last_search: Option<SearchResult>,
    pub current: SearchResult,

    /// Search matches that should be highlighted
    pub highlights: Vec<BufferRange>,
}

impl Search {
    pub fn save_last_search(&mut self) {
        let search = std::mem::take(&mut self.current);
        self.last_search = Some(search);
    }

    pub fn last_search(&self) -> Option<&SearchResult> {
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
            current: SearchResult::default(),
            highlights: vec![],
        }
    }
}
