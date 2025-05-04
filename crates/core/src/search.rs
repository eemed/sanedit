use crate::{BufferRange, Range};
use sanedit_buffer::{
    Bytes, PieceTreeSlice, SearchIter, SearchIterRev, Searcher as PTSearcher,
    SearcherRev as PTSearcherRev,
};
use sanedit_syntax::{CaptureIter, Regex};

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum SearchKind {
    /// default case sensitive search
    Default(bool),
    Regex,
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
            SearchKind::Regex => false,
        }
    }

    pub fn reverse(&mut self) {
        match self {
            SearchKind::Default(rev) => *rev = !*rev,
            _ => {}
        }
    }

    pub fn is_reversed(&self) -> bool {
        match self {
            SearchKind::Default(rev) => *rev,
            SearchKind::Regex => false,
        }
    }
}

#[derive(Debug)]
pub enum Searcher {
    /// Forwards search
    Forward(PTSearcher),

    /// Backwards search
    Rev(PTSearcherRev),

    /// Forward search
    Regex(Regex),
}

impl Searcher {
    pub fn new(pattern: &str, kind: SearchKind) -> anyhow::Result<Searcher> {
        match kind {
            SearchKind::Default(true) => Ok(Self::create_rev(pattern)),
            SearchKind::Default(false) => Ok(Self::create(pattern)),
            SearchKind::Regex => Self::create_regex(pattern),
        }
    }

    fn create_regex(patt: &str) -> anyhow::Result<Searcher> {
        let regex = Regex::new(patt)?;
        let searcher = Searcher::Regex(regex);
        Ok(searcher)
    }

    fn create(patt: &str) -> Searcher {
        let searcher = PTSearcher::new(patt.as_bytes());
        Searcher::Forward(searcher)
    }

    fn create_rev(patt: &str) -> Searcher {
        let searcher = PTSearcherRev::new(patt.as_bytes());
        Searcher::Rev(searcher)
    }

    pub fn find_iter<'a, 'b: 'a>(&'a self, slice: &'b PieceTreeSlice) -> MatchIter<'a, 'b> {
        match self {
            Searcher::Forward(s) => {
                let iter = s.find_iter(slice);
                MatchIter::Forward(iter)
            }
            Searcher::Rev(s) => {
                let iter = s.find_iter(slice);
                MatchIter::Rev(iter)
            }
            Searcher::Regex(regex) => {
                let bytes = slice.bytes();
                let iter = regex.captures(bytes);
                MatchIter::Regex(iter)
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
    Forward(SearchIter<'a, 'b>),
    Rev(SearchIterRev<'a, 'b>),
    Regex(CaptureIter<'a, Bytes<'a>>),
}

impl<'a, 'b> Iterator for MatchIter<'a, 'b> {
    type Item = SearchMatch;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            MatchIter::Forward(i) => {
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
            MatchIter::Regex(capture_iter) => {
                let caps = capture_iter.next()?;
                let cap = &caps[0];
                Some(SearchMatch {
                    range: cap.range().into(),
                })
            }
        }
    }
}
