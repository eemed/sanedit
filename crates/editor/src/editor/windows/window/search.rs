use sanedit_core::{BufferRange, SearchOptions};

#[derive(Debug, Default)]
pub(crate) struct SearchHighlights {
    /// Resulting highlights from the parse
    pub highlights: Vec<BufferRange>,

    /// How many changes were made when highlights were calculated
    pub changes_made: u32,

    /// Buffer range that was parsed
    pub buffer_range: BufferRange,
}

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
    ///
    /// Hooks will take care of highlighting,
    /// to trigger automatic highlightin set this to Some(Default::default())
    pub highlights: Option<SearchHighlights>,
}

impl Default for Search {
    fn default() -> Self {
        Search {
            on_line_char_search: None,
            current: SearchResult::default(),
            highlights: None,
        }
    }
}
