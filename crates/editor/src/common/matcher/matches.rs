use std::ops::Range;

use crate::editor::windows::SelectorOption;

/// A generic match option that contains bytes.
/// And a description of those bytes
#[derive(Debug, Clone)]
pub(crate) struct MatchOption {
    pub(crate) value: Vec<u8>,
    pub(crate) description: String,
}

impl std::hash::Hash for MatchOption {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
}

impl PartialEq for MatchOption {
    fn eq(&self, other: &Self) -> bool {
        self.value.eq(&other.value)
    }
}

impl Eq for MatchOption {}

impl MatchOption {
    pub fn new(value: &str) -> MatchOption {
        MatchOption {
            value: value.into(),
            description: String::new(),
        }
    }
}

impl From<String> for MatchOption {
    fn from(value: String) -> Self {
        MatchOption {
            value: value.into(),
            description: String::new(),
        }
    }
}

/// A matched and scored candidate
#[derive(Debug, Clone)]
pub(crate) struct Match {
    /// Matched value
    pub(crate) opt: MatchOption,
    /// Score of the match
    pub(crate) score: u32,

    /// Ranges of value string that were matched
    pub(crate) ranges: Vec<Range<usize>>,
}

impl Match {
    pub fn score(&self) -> u32 {
        self.score
    }

    pub fn ranges(&self) -> &[Range<usize>] {
        &self.ranges
    }
}

impl PartialEq for Match {
    fn eq(&self, other: &Self) -> bool {
        (self.score, &self.opt.value) == (other.score, &other.opt.value)
    }
}

impl Eq for Match {}

impl PartialOrd for Match {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        (self.score, &self.opt.value).partial_cmp(&(other.score, &other.opt.value))
    }
}

impl Ord for Match {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.score, &self.opt.value).cmp(&(other.score, &other.opt.value))
    }
}

impl From<Match> for SelectorOption {
    fn from(mat: Match) -> Self {
        let value = String::from_utf8_lossy(&mat.opt.value);
        let mut opt = SelectorOption::new(value.into(), mat.ranges, mat.score);
        opt.description = mat.opt.description;
        opt
    }
}
