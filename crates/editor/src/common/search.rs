use std::ops::Range;

use regex_cursor::{
    engines::meta::{FindMatches, Regex},
    Cursor, Input,
};
use sanedit_buffer::{
    Chunk, Chunks, PieceTreeSlice, SearchIter, SearchIterRev, Searcher, SearcherRev,
};

use crate::editor::windows::{SearchDirection, SearchKind};

pub(crate) enum PTSearcher {
    Regex(Regex),
    Forward(Searcher),
    Backwards(SearcherRev),
}

impl PTSearcher {
    pub fn new(term: &str, dir: SearchDirection, kind: SearchKind) -> anyhow::Result<PTSearcher> {
        use SearchDirection::*;
        use SearchKind::*;
        match (dir, kind) {
            (Forward, Regex) => Self::regex(term),
            (Backward, Regex) => todo!("Just search fwd and yield backwards?"),
            (Forward, _) => Ok(Self::forward(term)),
            (Backward, _) => Ok(Self::backward(term)),
        }
    }

    fn forward(term: &str) -> PTSearcher {
        let searcher = Searcher::new(term.as_bytes());
        PTSearcher::Forward(searcher)
    }

    fn backward(term: &str) -> PTSearcher {
        let searcher = SearcherRev::new(term.as_bytes());
        PTSearcher::Backwards(searcher)
    }

    fn regex(term: &str) -> anyhow::Result<PTSearcher> {
        let searcher = Regex::new(term)?;
        Ok(PTSearcher::Regex(searcher))
    }

    pub fn find_iter<'a, 'b: 'a>(&'a self, slice: &'b PieceTreeSlice) -> MatchIter<'a, 'b> {
        match self {
            PTSearcher::Regex(r) => {
                let len = slice.len();
                let chunks = slice.chunks();
                let chunk = chunks.get();
                let input = Input::new(PTRegexCursor { len, chunks, chunk });
                let iter = r.find_iter(input);
                MatchIter::Regex(iter)
            }
            PTSearcher::Forward(s) => {
                let iter = s.find_iter(slice);
                MatchIter::Forward(iter)
            }
            PTSearcher::Backwards(s) => {
                let iter = s.find_iter(slice);
                MatchIter::Backwards(iter)
            }
        }
    }
}

#[derive(Debug)]
pub(crate) struct SearchMatch {
    pub(crate) range: Range<usize>,
}

impl SearchMatch {
    pub fn range(&self) -> Range<usize> {
        self.range.clone()
    }
}

pub(crate) enum MatchIter<'a, 'b> {
    Regex(FindMatches<'a, PTRegexCursor<'b>>),
    Forward(SearchIter<'a, 'b>),
    Backwards(SearchIterRev<'a, 'b>),
}

impl<'a, 'b> Iterator for MatchIter<'a, 'b> {
    type Item = SearchMatch;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            MatchIter::Regex(i) => {
                let next = i.next()?;
                Some(SearchMatch {
                    range: next.range(),
                })
            }
            MatchIter::Forward(i) => {
                let next = i.next()?;
                Some(SearchMatch {
                    range: next.start..next.end,
                })
            }
            MatchIter::Backwards(i) => {
                let next = i.next()?;
                Some(SearchMatch {
                    range: next.start..next.end,
                })
            }
        }
    }
}

pub(crate) struct PTRegexCursor<'a> {
    len: usize,
    chunks: Chunks<'a>,
    chunk: Option<(usize, Chunk<'a>)>,
}

impl<'a> Cursor for PTRegexCursor<'a> {
    fn chunk(&self) -> &[u8] {
        match &self.chunk {
            Some((_, chk)) => chk.as_ref(),
            None => &[],
        }
    }

    fn advance(&mut self) -> bool {
        if let Some(chk) = self.chunks.next() {
            self.chunk = Some(chk);
            true
        } else {
            false
        }
    }

    fn backtrack(&mut self) -> bool {
        if let Some(chk) = self.chunks.prev() {
            self.chunk = Some(chk);
            true
        } else {
            false
        }
    }

    fn total_bytes(&self) -> Option<usize> {
        Some(self.len)
    }

    fn offset(&self) -> usize {
        match &self.chunk {
            Some((off, _)) => *off,
            None => 0,
        }
    }

    fn utf8_aware(&self) -> bool {
        true
    }
}
