use serde::{Deserialize, Serialize};

use crate::editor::themes::DEFAULT_THEME;

#[derive(Debug, Clone, Serialize, Deserialize, DocComment)]
#[serde(default)]
pub(crate) struct WindowConfig {
    /// Maximum prompt completions to show at once
    pub max_prompt_completions: usize,

    /// Maximum completions to show at once
    pub max_completions: usize,

    pub theme: String,

    /// Highlight LSP diagnostics
    pub highlight_syntax: bool,

    /// Highlight LSP diagnostics
    pub highlight_diagnostics: bool,

    /// Default persisted keys when creating a new window
    pub startup_persist_keys: String,
}

impl Default for WindowConfig {
    fn default() -> Self {
        WindowConfig {
            max_prompt_completions: 10,
            max_completions: 10,
            theme: DEFAULT_THEME.into(),
            highlight_syntax: true,
            highlight_diagnostics: true,
            startup_persist_keys: "esc".into(),
        }
    }
}
