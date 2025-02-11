use crate::{BufferRange, Range};
use sanedit_buffer::{
    PieceTreeSlice, SearchIter, SearchIterRev, Searcher as PTSearcher, SearcherRev as PTSearcherRev,
};

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum SearchKind {
    /// default case sensitive search
    Default(bool),
}

impl Default for SearchKind {
    fn default() -> Self {
        Self::Default(false)
    }
}

impl SearchKind {
    pub fn tag(&self) -> &str {
        match self {
            SearchKind::Default(true) => "rev",
            _ => "",
        }
    }

    pub fn can_reverse(&self) -> bool {
        match self {
            SearchKind::Default(_) => true,
        }
    }

    pub fn reverse(&mut self) {
        match self {
            SearchKind::Default(rev) => *rev = !*rev,
        }
    }

    pub fn is_reversed(&self) -> bool {
        match self {
            SearchKind::Default(rev) => *rev,
        }
    }
}

#[derive(Debug)]
pub enum Searcher {
    /// Forward search
    Default(PTSearcher),

    /// Backwards search
    Rev(PTSearcherRev),
}

impl Searcher {
    pub fn new(pattern: &str, kind: SearchKind) -> anyhow::Result<Searcher> {
        match kind {
            SearchKind::Default(true) => Ok(Self::create_rev(pattern)),
            SearchKind::Default(false) => Ok(Self::create(pattern)),
        }
    }

    fn create(patt: &str) -> Searcher {
        let searcher = PTSearcher::new(patt.as_bytes());
        Searcher::Default(searcher)
    }

    fn create_rev(patt: &str) -> Searcher {
        let searcher = PTSearcherRev::new(patt.as_bytes());
        Searcher::Rev(searcher)
    }

    pub fn find_iter<'a, 'b: 'a>(&'a self, slice: &'b PieceTreeSlice) -> MatchIter<'a, 'b> {
        match self {
            Searcher::Default(s) => {
                let iter = s.find_iter(slice);
                MatchIter::Default(iter)
            }
            Searcher::Rev(s) => {
                let iter = s.find_iter(slice);
                MatchIter::Rev(iter)
            }
        }
    }
}

#[derive(Debug)]
pub struct SearchMatch {
    range: BufferRange,
}

impl SearchMatch {
    pub fn range(&self) -> BufferRange {
        self.range.clone()
    }
}

pub enum MatchIter<'a, 'b> {
    Default(SearchIter<'a, 'b>),
    Rev(SearchIterRev<'a, 'b>),
}

impl<'a, 'b> Iterator for MatchIter<'a, 'b> {
    type Item = SearchMatch;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            MatchIter::Default(i) => {
                let next = i.next()?;
                Some(SearchMatch {
                    range: Range::new(next.start, next.end),
                })
            }
            MatchIter::Rev(i) => {
                let next = i.next()?;
                Some(SearchMatch {
                    range: Range::new(next.start, next.end),
                })
            }
        }
    }
}
