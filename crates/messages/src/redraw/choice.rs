use serde::{Deserialize, Serialize};

pub use sanedit_core::Range;

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct Choice {
    pub text: String,
    pub description: String,
    pub matches: Vec<Range<usize>>,
}
