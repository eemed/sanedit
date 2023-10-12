use sanedit_utils::sorted_vec::SortedVec;

pub(crate) type Options = SortedVec<Opt>;

#[derive(Debug, Clone, Default)]
pub(crate) struct Opt {
    pub(super) score: u32,
    pub(super) opt: String,
}

impl Opt {
    pub fn new(opt: String, score: u32) -> Opt {
        Opt { opt, score }
    }

    pub fn as_str(&self) -> &str {
        self.as_str()
    }

    pub fn score(&self) -> u32 {
        self.score
    }
}

impl PartialEq for Opt {
    fn eq(&self, other: &Self) -> bool {
        self.score == other.score
    }
}

impl Eq for Opt {}

impl PartialOrd for Opt {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.score.partial_cmp(&other.score)
    }
}

impl Ord for Opt {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.score.cmp(&other.score)
    }
}
