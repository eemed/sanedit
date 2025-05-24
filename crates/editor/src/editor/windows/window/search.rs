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
    highlights: SearchHighlights,
    show_highlights: bool,
}

impl Search {
    pub fn highlights(&self) -> Option<&SearchHighlights> {
        if self.show_highlights {
            Some(&self.highlights)
        } else {
            None
        }
    }

    pub fn highlights_mut(&mut self) -> Option<&mut SearchHighlights> {
        if self.show_highlights {
            Some(&mut self.highlights)
        } else {
            None
        }
    }

    pub fn set_highlights(&mut self, hls: SearchHighlights) {
        self.show_highlights = true;
        self.highlights = hls;
    }

    pub fn is_highlighting_enabled(&self) -> bool {
        self.show_highlights
    }

    pub fn enable_highlighting(&mut self) {
        self.show_highlights = true;
    }

    pub fn disable_highlighting(&mut self) {
        self.show_highlights = false;
        self.reset_highlighting();
    }

    pub fn reset_highlighting(&mut self) {
        self.highlights = Default::default();
    }
}

impl Default for Search {
    fn default() -> Self {
        Search {
            on_line_char_search: None,
            current: SearchResult::default(),
            show_highlights: false,
            highlights: Default::default(),
        }
    }
}
