use std::ops::Range;

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

    /// Use the search term as a regex
    Regex,
}

#[derive(Debug)]
pub(crate) struct Search {
    pub last_search: String,
    pub hl_matches: Vec<Range<usize>>,
    pub cmatch: Option<Range<usize>>,

    pub direction: SearchDirection,
    pub kind: SearchKind,
}

impl Search {
    pub fn new(msg: &str) -> Search {
        Search {
            last_search: String::new(),
            hl_matches: vec![],
            cmatch: None,
            direction: SearchDirection::Forward,
            kind: SearchKind::default(),
        }
    }
}

impl Default for Search {
    fn default() -> Self {
        Search::new("")
    }
}
