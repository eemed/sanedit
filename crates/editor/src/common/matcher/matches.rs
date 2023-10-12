use crate::editor::windows::Opt;

/// A matched and scored candidate
#[derive(Debug, Clone)]
pub(crate) struct Match {
    pub(crate) value: String,
    pub(crate) score: u32,
}

impl Match {
    pub fn as_str(&self) -> &str {
        self.value.as_str()
    }

    pub fn score(&self) -> u32 {
        self.score
    }
}

impl PartialEq for Match {
    fn eq(&self, other: &Self) -> bool {
        self.score == other.score
    }
}

impl Eq for Match {}

impl PartialOrd for Match {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.score.partial_cmp(&other.score)
    }
}

impl Ord for Match {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.score.cmp(&other.score)
    }
}

impl From<Match> for Opt {
    fn from(mat: Match) -> Self {
        Opt::new(mat.value, mat.score)
    }
}
