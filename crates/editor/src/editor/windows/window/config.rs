use serde::{Deserialize, Serialize};

use crate::editor::themes::DEFAULT_THEME;

use super::window_manager::WindowManager;

#[derive(Debug, Clone, Serialize, Deserialize, DocComment)]
#[serde(default)]
pub(crate) struct WindowConfig {
    /// Maximum prompt completions to show at once
    pub max_prompt_completions: usize,

    /// Maximum completions to show at once
    pub max_completions: usize,

    pub theme: String,

    /// Highlight syntax
    pub highlight_syntax: bool,

    /// Highlight LSP diagnostics
    pub highlight_diagnostics: bool,

    /// Automatically indent lines, and clear them from indent
    pub autoindent: bool,

    /// Automatically insert pairs on enter, works only with autoindent
    pub autopair: bool,

    /// Currently used window manager
    /// Options:
    ///     Auto: automatically detect window manager
    ///     Tmux
    pub window_manager: WindowManagerConfig,
}

impl Default for WindowConfig {
    fn default() -> Self {
        WindowConfig {
            max_prompt_completions: 15,
            max_completions: 15,
            theme: DEFAULT_THEME.into(),
            highlight_syntax: true,
            highlight_diagnostics: true,
            autoindent: true,
            autopair: true,
            window_manager: WindowManagerConfig::Auto,
        }
    }
}

/// How to open new windows
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) enum WindowManagerConfig {
    #[default]
    Auto,
    Tmux,
    Wezterm,
}

impl WindowManagerConfig {
    pub fn get(&self) -> WindowManager {
        if let Ok(prog) = std::env::var("TERM_PROGRAM") {
            return match prog.as_str() {
                "WezTerm" => WindowManager::wezterm(),
                "tmux" => WindowManager::tmux(),
                _ => WindowManager::None,
            };
        }

        if std::env::var("TMUX").is_ok() {
            return WindowManager::tmux();
        }

        if std::env::var("WEZTERM_PANE").is_ok() {
            return WindowManager::wezterm();
        }

        WindowManager::None
    }
}
