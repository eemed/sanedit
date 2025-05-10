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
    pub current: SearchResult,

    /// Search matches that should be highlighted
    pub highlights: Vec<BufferRange>,
}

impl Default for Search {
    fn default() -> Self {
        Search {
            on_line_char_search: None,
            current: SearchResult::default(),
            highlights: vec![],
        }
    }
}
