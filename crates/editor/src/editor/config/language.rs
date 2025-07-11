use std::{cell::OnceCell, path::Path, sync::Arc};

use serde::{Deserialize, Serialize};

use crate::{common::matcher::Choice, editor::snippets::Snippet};

use super::{buffers, read_toml};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ConfigSnippet {
    trigger: String,
    body: String,

    #[serde(skip)]
    loaded: OnceCell<Option<Snippet>>,
}

impl ConfigSnippet {
    pub fn get(&self) -> Option<Snippet> {
        self.loaded
            .get_or_init(|| match Snippet::new_trigger(&self.body, &self.trigger) {
                Ok(snip) => Some(snip),
                Err(e) => {
                    log::error!("Failed to load snippet: {e}");
                    None
                }
            })
            .clone()
    }
}

#[derive(Debug, Clone, Serialize, Default, Deserialize, DocComment)]
#[serde(default)]
pub(crate) struct LSPConfig {
    /// Command to run LSP
    pub command: String,

    /// Arguments to pass onto LSP command
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Default, Deserialize, DocComment)]
#[serde(default)]
pub(crate) struct LanguageConfig {
    /// Used to comment blocks of text
    pub comment: String,

    pub language_server: LSPConfig,

    pub buffer: buffers::BufferConfig,

    pub snippet: Vec<ConfigSnippet>,
}

impl LanguageConfig {
    pub fn new(config_path: &Path) -> LanguageConfig {
        match Self::try_new(config_path) {
            Ok(config) => config,
            Err(e) => {
                log::warn!("Failed to load language configuration, using default instead: {e}");
                LanguageConfig::default()
            }
        }
    }

    pub fn try_new(config_path: &Path) -> anyhow::Result<LanguageConfig> {
        read_toml::<LanguageConfig>(config_path)
    }

    pub fn snippets_as_choices(&self) -> Vec<Arc<Choice>> {
        let mut choices = vec![];
        for snip in &self.snippet {
            if let Some(loaded) = snip.get() {
                choices.push(Choice::from_snippet_trigger(loaded));
            }
        }

        choices
    }
}
