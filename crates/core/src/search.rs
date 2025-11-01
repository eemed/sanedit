use crate::{BufferRange, Range};
use anyhow::bail;
use sanedit_syntax::{CaptureIter, Finder, FinderIter, FinderIterRev, FinderRev, Regex, Source};

#[derive(Debug, Clone, Copy)]
pub struct SearchOptions {
    pub is_case_sensitive: bool,
    pub is_reversed: bool,
    pub is_regex: bool,
}

impl SearchOptions {
    pub fn from_pattern(pattern: &str) -> (SearchOptions, String) {
        let case_sensitive = pattern.chars().any(|ch| ch.is_ascii_uppercase());
        let is_regex = pattern.starts_with("/") && pattern.ends_with("/") && pattern.len() > 2;
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
    Finder(Box<Finder>),
    FinderRev(Box<FinderRev>),

    Regex(Regex),
    RegexRev(Regex),
}

impl Searcher {
    /// Create a new searched with specific type
    pub fn with_options(pattern: &str, options: &SearchOptions) -> anyhow::Result<Searcher> {
        if pattern.is_empty() {
            bail!("Empty pattern");
        }

        if options.is_regex && options.is_reversed {
            Self::create_regex_rev(pattern, options)
        } else if options.is_regex {
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

    fn create_regex_rev(patt: &str, _options: &SearchOptions) -> anyhow::Result<Searcher> {
        let regex = Regex::new(patt)?;
        let searcher = Searcher::RegexRev(regex);
        Ok(searcher)
    }

    fn create_regex(patt: &str, _options: &SearchOptions) -> anyhow::Result<Searcher> {
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
        Searcher::Finder(searcher.into())
    }

    fn create_rev(patt: &str, options: &SearchOptions) -> Searcher {
        let searcher = if options.is_case_sensitive || !patt.is_ascii() {
            FinderRev::new(patt.as_bytes())
        } else {
            FinderRev::new_case_insensitive(patt.as_bytes())
        };
        Searcher::FinderRev(searcher.into())
    }

    pub fn find_iter<'b, T: Source>(&self, source: &'b mut T) -> MatchIter<'_, 'b, T> {
        match self {
            Searcher::Regex(regex) => {
                let iter = regex.captures(source);
                MatchIter::Regex(iter)
            }
            Searcher::RegexRev(regex) => {
                let iter = regex.captures(source);
                MatchIter::RegexRev(iter)
            }
            Searcher::Finder(finder) => {
                let iter = finder.iter(source);
                MatchIter::Finder(iter)
            }
            Searcher::FinderRev(finder) => {
                let iter = finder.iter(source);
                MatchIter::FinderRev(iter)
            }
        }
    }

    pub fn options(&self) -> SearchOptions {
        let is_regex = matches!(self, Self::Regex(..));
        let (is_case_sensitive, is_reversed) = match self {
            Searcher::Regex(_) => (true, false),
            Searcher::RegexRev(_) => (true, true),
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
        self.range
    }
}

pub enum MatchIter<'a, 'b, T: Source> {
    Finder(FinderIter<'a, 'b, T>),
    FinderRev(FinderIterRev<'a, 'b, T>),
    Regex(CaptureIter<'a, 'b, T>),
    RegexRev(CaptureIter<'a, 'b, T>),
}

impl<'a, 'b, T: Source> Iterator for MatchIter<'a, 'b, T> {
    type Item = SearchMatch;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            MatchIter::Finder(iter) => {
                let start = iter.next()?;
                let len = iter.needle().len();
                Some(SearchMatch {
                    range: Range::from(start..start + len as u64),
                })
            }
            MatchIter::FinderRev(iter) => {
                let start = iter.next()?;
                let len = iter.needle().len();
                Some(SearchMatch {
                    range: Range::from(start..start + len as u64),
                })
            }
            MatchIter::Regex(capture_iter) => {
                let caps = capture_iter.next()?;
                let cap = caps.last().unwrap();
                Some(SearchMatch {
                    range: cap.range().into(),
                })
            }
            MatchIter::RegexRev(capture_iter) => {
                let caps = capture_iter.next_back()?;
                let cap = caps.last().unwrap();
                Some(SearchMatch {
                    range: cap.range().into(),
                })
            }
        }
    }
}
