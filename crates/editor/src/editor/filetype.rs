use std::path::Path;

use sanedit_core::Filetype;

use super::{config::FiletypeConfig, Map};

#[derive(Debug, Default)]
pub struct Filetypes {
    filetypes: Map<Filetype, FiletypeConfig>,
}

impl Filetypes {
    pub fn new() -> Filetypes {
        Filetypes {
            filetypes: Map::default(),
        }
    }

    pub fn get(&self, ft: &Filetype) -> Option<&FiletypeConfig> {
        self.filetypes.get(ft)
    }

    pub fn contains_key(&self, ft: &Filetype) -> bool {
        self.filetypes.contains_key(ft)
    }

    pub fn reload(&mut self, ft: &Filetype, path: &Path) -> anyhow::Result<()> {
        let ftc = FiletypeConfig::new(path);
        self.filetypes.insert(ft.clone(), ftc);
        Ok(())
    }

    pub fn load(&mut self, ft: &Filetype, path: &Path) -> anyhow::Result<()> {
        if self.filetypes.contains_key(ft) {
            return Ok(());
        }

        let ftc = FiletypeConfig::new(path);
        log::info!("FTC: {path:?}, {ftc:?}");
        self.filetypes.insert(ft.clone(), ftc);
        Ok(())
    }
}
