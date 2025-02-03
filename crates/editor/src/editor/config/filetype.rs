use std::{collections::HashMap, path::Path};

use serde::{Deserialize, Serialize};

use super::{buffers, KeymapLayer};

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
pub(crate) struct General {
    /// Used to comment blocks of text
    pub comment: String,
}

#[derive(Debug, Serialize, Default, Deserialize, DocComment)]
#[serde(default)]
pub(crate) struct FiletypeConfig {
    pub general: General,
    pub lsp: LSPConfig,
    /// Custom buffer options for this filetype, overrides global
    pub buffer: buffers::BufferConfig,
    /// Curstom keymap options for this filetype, added to global
    pub keymaps: HashMap<String, KeymapLayer>,
}

impl FiletypeConfig {
    pub fn new(path: &Path) -> FiletypeConfig {
        match Self::try_new(path) {
            Ok(config) => config,
            Err(e) => {
                log::warn!("Failed to load filetype configuration, using default instead: {e}");
                FiletypeConfig::default()
            }
        }
    }

    pub fn try_new(path: &Path) -> anyhow::Result<FiletypeConfig> {
        let builder = config::Config::builder().add_source(config::File::from(path));
        let config = builder.build()?.try_deserialize::<FiletypeConfig>()?;
        Ok(config)
    }
}
