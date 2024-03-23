use std::ops::Range;

use crate::editor::windows::SelectorOption;

/// A matched and scored candidate
#[derive(Debug, Clone)]
pub(crate) struct Match {
    /// Matched value
    pub(crate) value: String,
    /// Score of the match
    pub(crate) score: u32,

    /// Ranges of value string that were matched
    pub(crate) ranges: Vec<Range<usize>>,
}

impl Match {
    pub fn as_str(&self) -> &str {
        self.value.as_str()
    }

    pub fn score(&self) -> u32 {
        self.score
    }

    pub fn ranges(&self) -> &[Range<usize>] {
        &self.ranges
    }
}

impl PartialEq for Match {
    fn eq(&self, other: &Self) -> bool {
        (self.score, &self.value) == (other.score, &other.value)
    }
}

impl Eq for Match {}

impl PartialOrd for Match {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        (self.score, &self.value).partial_cmp(&(other.score, &other.value))
    }
}

impl Ord for Match {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.score, &self.value).cmp(&(other.score, &other.value))
    }
}

impl From<Match> for SelectorOption {
    fn from(mat: Match) -> Self {
        SelectorOption::new(mat.value, mat.ranges, mat.score)
    }
}
