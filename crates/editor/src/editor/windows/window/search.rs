use sanedit_core::BufferRange;

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SearchDirection {
    #[default]
    Forward,
    Backward,
}

impl SearchDirection {
    pub fn reverse(&self) -> SearchDirection {
        use SearchDirection::*;
        match self {
            Backward => Forward,
            Forward => Backward,
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Copy)]
pub(crate) enum SearchKind {
    /// Smartcase search
    /// If all lowercase => case insensitive
    /// otherwise case sensitive
    #[default]
    Smart,

    /// Use the search pattern as a regex
    Regex,
}

impl SearchKind {
    pub fn tag(&self) -> &str {
        match self {
            SearchKind::Smart => "",
            SearchKind::Regex => "regex",
        }
    }
}

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
            hl_matches: vec![],
            current_match: None,
        }
    }
}
