use crate::{BufferRange, Range};
use anyhow::bail;
use regex_cursor::{
    engines::meta::{FindMatches, Regex},
    regex_automata::{
        hybrid::dfa::{Cache, DFA},
        nfa::thompson,
    },
    Cursor, Input,
};
use sanedit_buffer::{
    Chunk, Chunks, PieceTreeSlice, SearchIter, SearchIterRev, Searcher, SearcherRev,
};

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchDirection {
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
pub enum SearchKind {
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

#[derive(Debug)]
pub enum PTSearcher {
    /// Regex search
    RegexFwd(Regex),

    /// Backwards regex search
    RegexBwd { bwd: DFA, fwd: DFA },

    /// Forward search
    Fwd(Searcher),

    /// Backwards search
    Bwd(SearcherRev),
}

impl PTSearcher {
    pub fn new(
        pattern: &str,
        dir: SearchDirection,
        kind: SearchKind,
    ) -> anyhow::Result<PTSearcher> {
        if pattern.is_empty() {
            bail!("Searcher empty pattern");
        }

        use SearchDirection::*;
        use SearchKind::*;
        match (dir, kind) {
            (Forward, Regex) => Self::regex_fwd(pattern),
            (Backward, Regex) => Self::regex_bwd(pattern),
            (Forward, _) => Ok(Self::fwd(pattern)),
            (Backward, _) => Ok(Self::bwd(pattern)),
        }
    }

    fn fwd(patt: &str) -> PTSearcher {
        let searcher = Searcher::new(patt.as_bytes());
        PTSearcher::Fwd(searcher)
    }

    fn bwd(patt: &str) -> PTSearcher {
        let searcher = SearcherRev::new(patt.as_bytes());
        PTSearcher::Bwd(searcher)
    }

    fn regex_fwd(patt: &str) -> anyhow::Result<PTSearcher> {
        let searcher = Regex::new(patt)?;
        Ok(PTSearcher::RegexFwd(searcher))
    }

    fn regex_bwd(patt: &str) -> anyhow::Result<PTSearcher> {
        let dfa_fwd = DFA::builder()
            .thompson(thompson::Config::new())
            .build(patt)?;
        let dfa_bwd = DFA::builder()
            .thompson(thompson::Config::new().reverse(true))
            .build(patt)?;
        Ok(PTSearcher::RegexBwd {
            bwd: dfa_bwd,
            fwd: dfa_fwd,
        })
    }

    pub fn find_iter<'a, 'b: 'a>(&'a self, slice: &'b PieceTreeSlice) -> MatchIter<'a, 'b> {
        match self {
            PTSearcher::RegexFwd(r) => {
                let input = to_input(slice);
                let iter = r.find_iter(input);
                MatchIter::Regex(iter)
            }
            PTSearcher::RegexBwd { bwd, fwd } => {
                let bwd_cache = bwd.create_cache();
                let fwd_cache = fwd.create_cache();
                MatchIter::RegexBwd(Box::new(RegexBwd {
                    fwd,
                    bwd,
                    fwd_cache,
                    bwd_cache,
                    slice: slice.clone(),
                }))
            }
            PTSearcher::Fwd(s) => {
                let iter = s.find_iter(slice);
                MatchIter::Forward(iter)
            }
            PTSearcher::Bwd(s) => {
                let iter = s.find_iter(slice);
                MatchIter::Backwards(iter)
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

pub struct RegexBwd<'a, 'b> {
    fwd: &'a DFA,
    fwd_cache: Cache,
    bwd: &'a DFA,
    bwd_cache: Cache,
    slice: PieceTreeSlice<'b>,
}

pub enum MatchIter<'a, 'b> {
    RegexBwd(Box<RegexBwd<'a, 'b>>),
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
                let start = next.start() as u64;
                let end = next.end() as u64;
                Some(SearchMatch {
                    range: Range::new(start, end),
                })
            }
            MatchIter::Forward(i) => {
                let next = i.next()?;
                Some(SearchMatch {
                    range: Range::new(next.start, next.end),
                })
            }
            MatchIter::Backwards(i) => {
                let next = i.next()?;
                Some(SearchMatch {
                    range: Range::new(next.start, next.end),
                })
            }
            // Find next match and update slice to not search the same thing
            // again
            MatchIter::RegexBwd(_) => match self.regex_bwd_next() {
                Some(mat) => {
                    let MatchIter::RegexBwd(r) = self else {
                        unreachable!()
                    };
                    r.slice = r.slice.slice(..mat.range.start);
                    Some(mat)
                }
                None => {
                    let MatchIter::RegexBwd(r) = self else {
                        unreachable!()
                    };
                    r.slice = r.slice.slice(0..0);
                    None
                }
            },
        }
    }
}

impl<'a, 'b> MatchIter<'a, 'b> {
    fn regex_bwd_next(&mut self) -> Option<SearchMatch> {
        let MatchIter::RegexBwd(r) = self else {
            unreachable!("Called regex_bwd_next without being the variant")
        };

        // Find the start position of the match
        let mut input = to_input(&r.slice);
        let start =
            regex_cursor::engines::hybrid::try_search_rev(r.bwd, &mut r.bwd_cache, &mut input)
                .ok()
                .flatten()?;
        let off = start.offset() as u64;

        // Find the end position of the match
        let match_slice = r.slice.slice(off..);
        let mut finput = to_input(&match_slice);
        let end =
            regex_cursor::engines::hybrid::try_search_fwd(r.fwd, &mut r.fwd_cache, &mut finput)
                .ok()
                .flatten()?;
        let end_off = end.offset() as u64;

        let slice_start = r.slice.start() + off;
        let slice_end = slice_start + end_off;

        Some(SearchMatch {
            range: Range::new(slice_start, slice_end),
        })
    }
}

fn to_input<'s>(slice: &'s PieceTreeSlice) -> Input<PTRegexCursor<'s>> {
    let len = slice.len();
    let chunks = slice.chunks();
    let chunk = chunks.get();
    Input::new(PTRegexCursor { len, chunks, chunk })
}

pub struct PTRegexCursor<'a> {
    len: u64,
    chunks: Chunks<'a>,
    chunk: Option<(u64, Chunk<'a>)>,
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
        Some(self.len as usize)
    }

    fn offset(&self) -> usize {
        match &self.chunk {
            Some((off, _)) => *off as usize,
            None => 0,
        }
    }

    fn utf8_aware(&self) -> bool {
        true
    }
}
