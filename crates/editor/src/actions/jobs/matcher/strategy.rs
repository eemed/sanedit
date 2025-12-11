use std::sync::Arc;

use sanedit_core::Range;
use sanedit_syntax::{Finder, GitGlob};

trait MatchFn: Send + Sync {
    fn is_match(&self, opt: &str, results: &mut Vec<Range<usize>>);
}

/// Matches anywhere
impl MatchFn for Finder {
    fn is_match(&self, mut opt: &str, results: &mut Vec<Range<usize>>) {
        for start in self.iter(&mut opt) {
            let start = start as usize;
            results.push(Range::from(start..start + self.needle().len()));
        }
    }
}

/// Matches prefixes
pub(crate) struct Prefix {
    is_case_sensitive: bool,
    term: String,
}

impl MatchFn for Prefix {
    fn is_match(&self, opt: &str, results: &mut Vec<Range<usize>>) {
        if opt.len() < self.term.len() {
            return;
        }

        let result = if self.is_case_sensitive {
            opt.starts_with(&self.term)
        } else {
            let mut result = true;
            let tbytes = self.term.as_bytes();
            let obytes = opt.as_bytes();
            for i in 0..self.term.len() {
                let low = obytes[i].to_ascii_lowercase();
                if tbytes[i] != low {
                    result = false;
                    break;
                }
            }
            result
        };

        if result {
            results.push(Range::from(0..self.term.len()));
        }
    }
}

/// Matches prefixes
pub(crate) struct Glob {
    glob: GitGlob,
}

impl MatchFn for Glob {
    fn is_match(&self, opt: &str, results: &mut Vec<Range<usize>>) {
        if self.glob.is_match(opt.as_bytes()) {
            results.push(Range::from(0..opt.len()));
        }
    }
}

#[derive(Clone)]
pub(crate) struct MultiMatcher {
    is_empty: bool,
    matchers: Arc<Vec<Box<dyn MatchFn>>>,
}

impl MultiMatcher {
    pub fn is_match(&self, opt: &str, results: &mut Vec<Range<usize>>) {
        let start = results.len();
        let mut current = start;
        for mat in self.matchers.as_ref() {
            mat.is_match(opt, results);
            // If we dont find a match for term consider this filtered
            if current == results.len() {
                results.truncate(start);
                break;
            }

            current = results.len();
        }
    }

    pub fn is_empty(&self) -> bool {
        self.is_empty
    }
}

/// Where to match
///
/// Prefix matches from the start
/// Any matches anywhre
#[derive(Debug, Clone, Copy, Default)]
pub enum MatchStrategy {
    /// Match in any position
    /// and split term by whitespace and search each term separately
    #[default]
    Default,

    /// Match the prefix
    Prefix,

    /// The pattern is a glob
    Glob,
}

impl MatchStrategy {
    pub fn get_match_func(&self, terms: &[String], case_sensitive: bool) -> MultiMatcher {
        let mut matchers: Vec<Box<dyn MatchFn>> = Vec::with_capacity(terms.len());
        match self {
            MatchStrategy::Default => {
                for term in terms {
                    let finder = if case_sensitive {
                        Finder::new(term.as_str().as_bytes())
                    } else {
                        Finder::new_case_insensitive(term.as_str().as_bytes())
                    };
                    matchers.push(Box::new(finder));
                }
            }
            MatchStrategy::Prefix => {
                for term in terms {
                    let pfix = Prefix {
                        is_case_sensitive: case_sensitive,
                        term: term.clone(),
                    };
                    matchers.push(Box::new(pfix));
                }
            }
            MatchStrategy::Glob => {
                for term in terms {
                    if let Ok(glob) = GitGlob::new(term) {
                        matchers.push(Box::new(Glob { glob }));
                    }
                }
            }
        }

        MultiMatcher {
            is_empty: matchers.is_empty(),
            matchers: Arc::new(matchers),
        }
    }

    /// Whether to split term from whitespace, and match using all of them
    /// separately
    pub fn split(&self) -> bool {
        match self {
            MatchStrategy::Default => true,
            MatchStrategy::Prefix => false,
            MatchStrategy::Glob => false,
        }
    }
}
