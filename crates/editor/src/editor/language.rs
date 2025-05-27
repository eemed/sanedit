use std::path::Path;

use sanedit_core::Language;

use super::{config::LanguageConfig, Map};

#[derive(Debug, Default)]
pub struct Languages {
    languages: Map<Language, LanguageConfig>,
}

impl Languages {
    pub fn get(&self, ft: &Language) -> Option<&LanguageConfig> {
        self.languages.get(ft)
    }

    pub fn contains_key(&self, ft: &Language) -> bool {
        self.languages.contains_key(ft)
    }

    pub fn reload(&mut self, ft: &Language, path: &Path) -> anyhow::Result<()> {
        let ftc = LanguageConfig::new(path);
        self.languages.insert(ft.clone(), ftc);
        Ok(())
    }

    pub fn load(&mut self, ft: &Language, path: &Path) -> anyhow::Result<()> {
        if self.contains_key(ft) {
            return Ok(());
        }

        let ftc = LanguageConfig::new(path);
        self.languages.insert(ft.clone(), ftc);
        Ok(())
    }
}
