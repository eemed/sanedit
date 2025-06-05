use std::sync::{atomic::AtomicBool, Arc};

use crate::{BufferRange, Range};
use anyhow::bail;
use sanedit_buffer::{Bytes, PieceTreeSlice};
use sanedit_syntax::{CaptureIter, Finder, FinderIter, FinderIterRev, FinderRev, Regex};

#[derive(Debug, Clone, Copy)]
pub struct SearchOptions {
    pub is_case_sensitive: bool,
    pub is_reversed: bool,
    pub is_regex: bool,
}

impl SearchOptions {
    pub fn from_pattern(pattern: &str) -> (SearchOptions, String) {
        let case_sensitive = pattern.chars().any(|ch| ch.is_ascii_uppercase());
        let is_regex = pattern.starts_with("/") && pattern.ends_with("/") && pattern.len() >= 2;
        let options = SearchOptions {
            is_case_sensitive: is_regex || case_sensitive,
            is_reversed: false,
            is_regex,
        };

        let pattern = if options.is_regex {
            &pattern[1..pattern.len() - 1]
        } else {
            pattern
        };

        (options, pattern.into())
    }

    pub fn tag(&self) -> String {
        let mut result = String::new();
        if !self.is_case_sensitive {
            result.push('i');
        }

        if self.is_reversed {
            result.push('r');
        }

        if self.is_regex {
            result.push('R');
        }

        result
    }
}

impl Default for SearchOptions {
    fn default() -> Self {
        SearchOptions {
            is_case_sensitive: true,
            is_reversed: false,
            is_regex: false,
        }
    }
}

#[derive(Debug)]
pub enum Searcher {
    Finder(Finder),
    FinderRev(FinderRev),

    /// Forward search
    Regex(Regex),
}

impl Searcher {
    /// Create a new searched with specific type
    pub fn with_options(pattern: &str, options: &SearchOptions) -> anyhow::Result<Searcher> {
        if pattern.is_empty() {
            bail!("Empty pattern");
        }

        if options.is_regex {
            Self::create_regex(pattern, options)
        } else if options.is_reversed {
            Ok(Self::create_rev(pattern, options))
        } else {
            Ok(Self::create(pattern, options))
        }
    }

    /// Creates a forward searcher.
    /// Search regex if formatted like /<pattern>/
    /// Otherwise search literal string
    /// If contains uppercase letters search is case sensitive if only lowercase its case insensitive
    pub fn new(pattern: &str) -> anyhow::Result<(Searcher, String)> {
        let (options, pattern) = SearchOptions::from_pattern(pattern);
        let searcher = Self::with_options(&pattern, &options)?;
        Ok((searcher, pattern))
    }

    fn create_regex(patt: &str, _options: &SearchOptions) -> anyhow::Result<Searcher> {
        // TODO currently options not supported

        let regex = Regex::new(patt)?;
        let searcher = Searcher::Regex(regex);
        Ok(searcher)
    }

    fn create(patt: &str, options: &SearchOptions) -> Searcher {
        let searcher = if options.is_case_sensitive || !patt.is_ascii() {
            Finder::new(patt.as_bytes())
        } else {
            Finder::new_case_insensitive(patt.as_bytes())
        };
        Searcher::Finder(searcher)
    }

    fn create_rev(patt: &str, options: &SearchOptions) -> Searcher {
        let searcher = if options.is_case_sensitive || !patt.is_ascii() {
            FinderRev::new(patt.as_bytes())
        } else {
            FinderRev::new_case_insensitive(patt.as_bytes())
        };
        Searcher::FinderRev(searcher)
    }

    pub fn find_iter<'a, 'b: 'a>(&'a self, slice: &'b PieceTreeSlice) -> MatchIter<'a> {
        self.find_iter_stoppable(slice, Arc::new(AtomicBool::new(false)))
    }

    pub fn find_iter_stoppable<'a, 'b: 'a>(
        &'a self,
        slice: &'b PieceTreeSlice,
        stop: Arc<AtomicBool>,
    ) -> MatchIter<'a> {
        match self {
            Searcher::Regex(regex) => {
                let bytes = slice.bytes();
                let iter = regex.captures((bytes, stop));
                MatchIter::Regex(iter)
            }
            Searcher::Finder(finder) => {
                let bytes = slice.bytes();
                let iter = finder.iter((bytes, stop));
                MatchIter::Finder(iter)
            }
            Searcher::FinderRev(finder) => {
                let bytes = slice.bytes();
                let iter = finder.iter((bytes, stop));
                MatchIter::FinderRev(iter)
            }
        }
    }

    pub fn options(&self) -> SearchOptions {
        let is_regex = matches!(self, Self::Regex(..));
        let (is_case_sensitive, is_reversed) = match self {
            Searcher::Regex(_) => (false, false),
            Searcher::Finder(finder) => (finder.is_case_sensitive(), false),
            Searcher::FinderRev(finder) => (finder.is_case_sensitive(), true),
        };

        SearchOptions {
            is_case_sensitive,
            is_reversed,
            is_regex,
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

pub enum MatchIter<'a> {
    Finder(FinderIter<'a, (Bytes<'a>, Arc<AtomicBool>)>),
    FinderRev(FinderIterRev<'a, (Bytes<'a>, Arc<AtomicBool>)>),
    Regex(CaptureIter<'a, (Bytes<'a>, Arc<AtomicBool>)>),
}

impl<'a> Iterator for MatchIter<'a> {
    type Item = SearchMatch;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            MatchIter::Finder(iter) => {
                let start = iter.next()?;
                let len = iter.pattern_len();
                Some(SearchMatch {
                    range: Range::new(start, start + len as u64),
                })
            }
            MatchIter::FinderRev(iter) => {
                let start = iter.next()?;
                let len = iter.pattern_len();
                Some(SearchMatch {
                    range: Range::new(start, start + len as u64),
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
