use std::ops::Range;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Choice {
    pub(super) score: u32,
    /// Underlying option
    pub(super) value: Vec<u8>,

    /// Matched ranges
    pub(super) matches: Vec<Range<usize>>,

    /// Additional description of the option
    pub(super) description: String,
}

impl Choice {
    pub fn new(opt: &[u8], matches: Vec<Range<usize>>, score: u32, description: &str) -> Choice {
        Choice {
            value: opt.into(),
            score,
            description: description.into(),
            matches,
        }
    }

    pub fn to_str_lossy(&self) -> std::borrow::Cow<str> {
        String::from_utf8_lossy(&self.value)
    }

    pub fn value_raw(&self) -> &[u8] {
        &self.value
    }

    pub fn score(&self) -> u32 {
        self.score
    }

    pub fn matches(&self) -> &[Range<usize>] {
        &self.matches
    }

    pub fn description(&self) -> &str {
        &self.description
    }
}

impl PartialOrd for Choice {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(&other))
    }
}

impl Ord for Choice {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.score, &self.value).cmp(&(other.score, &other.value))
    }
}
