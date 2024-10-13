use serde::{Deserialize, Serialize};

use crate::Range;

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Default, Serialize, Deserialize)]
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

    pub fn rescore(&mut self, score: u32) {
        self.score = score;
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
