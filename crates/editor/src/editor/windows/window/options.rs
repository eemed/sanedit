use documented::DocumentedFields;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, DocumentedFields)]
#[serde(default)]
pub(crate) struct Options {
    ///
    /// Maximum prompt completions to show at once
    pub max_prompt_completions: usize,

    ///
    /// Maximum completions to show at once
    pub max_completions: usize,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            max_prompt_completions: 10,
            max_completions: 10,
        }
    }
}
