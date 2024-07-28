use std::path::Path;

use documented::DocumentedFields;
use serde::{Deserialize, Serialize};
use toml_edit::{
    ser::to_document,
    visit_mut::{visit_table_like_kv_mut, VisitMut},
    Item, KeyMut,
};

use crate::{common::indent::Indent, editor};

use super::{buffers, windows};

#[derive(Debug, Default, Serialize, Deserialize, DocumentedFields)]
pub(crate) struct Config {
    ///
    /// Sanedit configuration
    ///
    /// Configuration can be overridden using a sanedit-project.toml file in project
    /// root. The file should be in the same format as this one and
    /// configuration may be partially updated
    ///
    pub editor: editor::Options,
    pub window: windows::Options,
    pub file: buffers::Options,
}

pub(crate) const PROJECT_CONFIG: &str = "sanedit-project.toml";

pub(crate) fn read_config(config_path: &Path, working_dir: &Path) -> anyhow::Result<Config> {
    let mut local = working_dir.to_path_buf();
    local.push(PROJECT_CONFIG);

    let config = config::Config::builder()
        .add_source(config::File::from(config_path))
        .add_source(config::File::from(local))
        .build()?;

    let config = config.try_deserialize::<Config>()?;

    Ok(config)
}

pub(crate) fn serialize_default_configuration(path: &Path) {
    let config = Config::default();
    let mut doc = to_document(&config).unwrap().to_owned();

    let mut visitor = Formatter {
        state: VisitState::Config,
    };
    visitor.visit_document_mut(&mut doc);

    log::info!("{}", doc.to_string());
}

struct Formatter {
    state: VisitState,
}

impl VisitMut for Formatter {
    fn visit_table_like_kv_mut(&mut self, mut key: KeyMut<'_>, node: &mut Item) {
        if node.is_inline_table() {
            let item = std::mem::replace(node, Item::None);
            if let Ok(table) = item.into_table() {
                *node = Item::Table(table);
            }
        }

        let doc = {
            let keyname = key.get();
            match self.state {
                VisitState::Config => Config::get_field_docs(keyname),
                VisitState::Editor => editor::Options::get_field_docs(keyname),
                VisitState::Window => windows::Options::get_field_docs(keyname),
                VisitState::File => buffers::Options::get_field_docs(keyname),
                VisitState::Indent => Indent::get_field_docs(keyname),
                VisitState::Irrelevant => {
                    Err(documented::Error::NoDocComments("irrelevant".into()))
                }
            }
        };

        if let Ok(doc) = doc {
            let decor = key.leaf_decor_mut();
            log::info!("Decor: {:?}", decor.prefix());
            let mut comment = String::from("\n");
            for line in doc.lines() {
                let line = format!("# {line}\n");
                comment.push_str(&line);
            }
            decor.set_prefix(comment);
        }

        let keyname = key.get();
        let old = self.state;
        let new = self.state.descend(keyname);
        if new != VisitState::Irrelevant {
            self.state = new;
        }
        log::info!("keyname: {keyname} old: {old:?} new: {new:?}");

        // Recurse further into the document tree.
        visit_table_like_kv_mut(self, key, node);

        self.state = old;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum VisitState {
    Config,
    Editor,
    Window,
    File,
    Indent,
    Irrelevant,
}

impl VisitState {
    pub fn descend(self, key: &str) -> Self {
        match (self, key) {
            (VisitState::Config, "editor") => VisitState::Editor,
            (VisitState::Config, "window") => VisitState::Window,
            (VisitState::Config, "file") => VisitState::File,
            (VisitState::File, "indent") => VisitState::Indent,
            _ => VisitState::Irrelevant,
        }
    }
}
