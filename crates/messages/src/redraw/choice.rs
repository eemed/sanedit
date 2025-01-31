use sanedit_core::Range;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct Choice {
    pub text: String,
    pub description: String,
    pub matches: Vec<Range<usize>>,
}
