use documented::DocumentedFields;
use serde::{Deserialize, Serialize};

use crate::editor::themes::DEFAULT_THEME;

#[derive(Debug, Clone, Serialize, Deserialize, DocumentedFields)]
#[serde(default)]
pub(crate) struct WindowConfig {
    /// Maximum prompt completions to show at once
    pub max_prompt_completions: usize,

    /// Maximum completions to show at once
    pub max_completions: usize,

    pub theme: String,
}

impl Default for WindowConfig {
    fn default() -> Self {
        WindowConfig {
            max_prompt_completions: 10,
            max_completions: 10,
            theme: DEFAULT_THEME.into(),
        }
    }
}
