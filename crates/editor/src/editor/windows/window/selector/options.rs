use sanedit_utils::sorted_vec::SortedVec;

pub(crate) type Options = SortedVec<SelectorOption>;

#[derive(Debug, Clone, Default)]
pub(crate) struct SelectorOption {
    pub(super) score: u32,
    pub(super) opt: String,
}

impl SelectorOption {
    pub fn new(opt: String, score: u32) -> SelectorOption {
        SelectorOption { opt, score }
    }

    pub fn as_str(&self) -> &str {
        self.opt.as_str()
    }

    pub fn score(&self) -> u32 {
        self.score
    }
}

impl PartialEq for SelectorOption {
    fn eq(&self, other: &Self) -> bool {
        self.score == other.score
    }
}

impl Eq for SelectorOption {}

impl PartialOrd for SelectorOption {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.score.partial_cmp(&other.score)
    }
}

impl Ord for SelectorOption {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.score.cmp(&other.score)
    }
}
