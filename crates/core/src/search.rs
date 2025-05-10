use crate::{BufferRange, Range};
use anyhow::bail;
use sanedit_buffer::{
    Bytes, PieceTreeSlice, SearchIter, SearchIterRev, Searcher as PTSearcher,
    SearcherRev as PTSearcherRev,
};
use sanedit_syntax::{CaptureIter, Regex};

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
    /// Forwards search
    Forward(PTSearcher),

    /// Backwards search
    Rev(PTSearcherRev),

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

    fn create_regex(patt: &str, options: &SearchOptions) -> anyhow::Result<Searcher> {
        // TODO currently options not supported

        let regex = Regex::new(patt)?;
        let searcher = Searcher::Regex(regex);
        Ok(searcher)
    }

    fn create(patt: &str, options: &SearchOptions) -> Searcher {
        let searcher = if options.is_case_sensitive {
            PTSearcher::new(patt.as_bytes())
        } else {
            PTSearcher::new_ascii_case_insensitive(patt)
                .unwrap_or_else(|| PTSearcher::new(patt.as_bytes()))
        };
        Searcher::Forward(searcher)
    }

    fn create_rev(patt: &str, options: &SearchOptions) -> Searcher {
        let searcher = if options.is_case_sensitive {
            PTSearcherRev::new(patt.as_bytes())
        } else {
            PTSearcherRev::new_ascii_case_insensitive(patt)
                .unwrap_or_else(|| PTSearcherRev::new(patt.as_bytes()))
        };
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

    pub fn options(&self) -> SearchOptions {
        let is_regex = matches!(self, Self::Regex(..));
        let (is_case_sensitive, is_reversed) = match self {
            Searcher::Forward(s) => (s.is_case_sensitive(), false),
            Searcher::Rev(s) => (s.is_case_sensitive(), true),
            Searcher::Regex(_) => (false, false),
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
