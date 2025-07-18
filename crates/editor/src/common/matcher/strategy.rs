use sanedit_core::Range;

pub(crate) type MatchFn = fn(&str, &str) -> Option<Range<usize>>;

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
}

impl MatchStrategy {
    pub fn get(&self) -> MatchFn {
        match self {
            MatchStrategy::Default => default_match,
            MatchStrategy::Prefix => prefix_match,
        }
    }

    /// Whether to split term from whitespace, and match using all of them
    /// separately
    pub fn split(&self) -> bool {
        match self {
            MatchStrategy::Default => true,
            MatchStrategy::Prefix => false,
        }
    }
}

/// Default match function
/// matches if term is found anywhere on the searched string
fn default_match(me: &str, term: &str) -> Option<Range<usize>> {
    let start = me.find(term)?;
    Some(Range::new(start, start + term.len()))
}

/// Prefix match function
/// matches only if searched string starts with term
fn prefix_match(me: &str, term: &str) -> Option<Range<usize>> {
    if me.starts_with(term) {
        Some(Range::new(0, term.len()))
    } else {
        None
    }
}
