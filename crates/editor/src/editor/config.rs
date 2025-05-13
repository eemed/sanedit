mod default;
mod filetype;

use std::{path::Path, sync::Arc};

use filetype::ConfigSnippet;
use sanedit_buffer::utf8::EndOfLine;
use sanedit_messages::key::{try_parse_keyevents, KeyEvent};
use serde::{Deserialize, Serialize};
use toml_edit::{
    ser::to_document,
    visit_mut::{visit_table_like_kv_mut, VisitMut},
    Item, KeyMut,
};

use crate::{
    actions::{find_by_name, window::change_keymap, Action, ActionResult},
    common::matcher::Choice,
    editor::{self, buffers::EndOfLineDef},
};

use super::{
    buffers,
    keymap::{KeymapKind, Layer},
    windows,
};
use rustc_hash::FxHashMap;

use super::Map;
pub(crate) use filetype::{FiletypeConfig, LSPConfig};

#[derive(Debug, Serialize, Deserialize, DocComment)]
#[serde(default)]
pub(crate) struct Config {
    #[serde(flatten)]
    pub editor: editor::EditorConfig,

    #[serde(flatten)]
    pub window: windows::WindowConfig,

    pub keymaps: Map<String, KeymapLayer>,

    pub snippets: Map<String, ConfigSnippet>,
}

impl Config {
    pub fn new(config_path: &Path, working_dir: &Path) -> Config {
        match Self::try_new(config_path, working_dir) {
            Ok(config) => config,
            Err(e) => {
                log::warn!("Failed to load configuration, using default instead: {e}");
                Config::default()
            }
        }
    }

    pub fn try_new(config_path: &Path, _working_dir: &Path) -> anyhow::Result<Config> {
        let builder = config::Config::builder().add_source(config::File::from(config_path));
        let config = builder.build()?.try_deserialize::<Config>()?;
        Ok(config)
    }

    pub(crate) fn serialize_default_configuration(path: &Path) -> anyhow::Result<()> {
        use std::io::Write;

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let config = Config::default();
        let mut doc = to_document(&config).unwrap().to_owned();

        let mut visitor = Formatter {
            state: VisitState::Config,
            start_of_file: true,
        };
        visitor.visit_document_mut(&mut doc);

        let default_config = doc.to_string();
        let mut file = std::fs::File::create_new(path)?;
        file.write_all(default_config.as_bytes())?;

        Ok(())
    }

    pub fn snippets_as_choices(&self) -> Vec<Arc<Choice>> {
        let mut choices = vec![];
        for (_name, snip) in &self.snippets {
            if let Some(loaded) = snip.get() {
                choices.push(Choice::from_snippet_trigger(loaded));
            }
        }

        choices
    }
}

struct Formatter {
    state: VisitState,
    start_of_file: bool,
}

impl VisitMut for Formatter {
    fn visit_table_like_kv_mut(&mut self, mut key: KeyMut<'_>, node: &mut Item) {
        // Format inline tables to normal tables
        if node.is_inline_table() {
            let item = std::mem::replace(node, Item::None);
            if let Ok(mut table) = item.into_table() {
                // Hide tables without any entries
                table.set_implicit(true);
                *node = Item::Table(table);
            }
        } else if self.state == VisitState::Keymaps && node.is_array() {
            if let Some(arr) = node.as_array_mut() {
                arr.set_trailing_comma(false);

                for val in arr.iter_mut() {
                    val.decor_mut().set_prefix("\n    ");
                }

                if let Some(last) = arr.iter_mut().last() {
                    last.decor_mut().set_suffix(",\n");
                }
            }
        }

        let doc = {
            let keyname = key.get();
            match self.state {
                VisitState::Config => Config::doc_comment(keyname)
                    .or(editor::EditorConfig::doc_comment(keyname))
                    .or(windows::WindowConfig::doc_comment(keyname)),
                // VisitState::Editor => {}
                // VisitState::Window => {}
                _ => None,
                // _ => Err(documented::Error::NoDocComments("irrelevant".into())),
            }
        };

        // Add docstrings as comments
        if let Some(doc) = doc {
            let top = if self.start_of_file { "" } else { "\n" };
            let mut comment = String::from(top);
            for line in doc.lines() {
                if line.is_empty() {
                    comment.push_str("#\n");
                } else {
                    let line = format!("# {line}\n");
                    comment.push_str(&line);
                }
            }

            match node {
                // Add the comment to the table instead because of
                // https://github.com/toml-rs/toml/issues/691
                Item::Table(table) => {
                    let decor = table.decor_mut();
                    decor.set_prefix(comment);
                }
                _ => {
                    let decor = key.leaf_decor_mut();
                    decor.set_prefix(comment);
                }
            }
        }

        self.start_of_file = false;
        let keyname = key.get();
        let old = self.state;
        let new = self.state.descend(keyname);
        if new != VisitState::Irrelevant {
            self.state = new;
        }

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
    Keymaps,
    Maps,
    Irrelevant,
}

impl VisitState {
    pub fn descend(self, key: &str) -> Self {
        match (self, key) {
            (VisitState::Config, "editor") => VisitState::Editor,
            (VisitState::Config, "window") => VisitState::Window,
            (VisitState::Config, "keymaps") => VisitState::Keymaps,
            (VisitState::Keymaps, "maps") => VisitState::Maps,
            _ => VisitState::Irrelevant,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, DocComment)]
#[serde(default)]
pub(crate) struct EditorConfig {
    /// Large file threshold in bytes
    pub big_file_threshold_bytes: u64,

    /// Directories to ignore, when opening files etc.
    pub ignore_directories: Vec<String>,

    /// Default shell command
    pub shell: String,

    /// Autodetect eol from file
    pub detect_eol: bool,

    /// Default eol
    #[serde(with = "EndOfLineDef")]
    pub eol: EndOfLine,

    /// Autodetect indentation from file
    pub detect_indent: bool,

    /// Filetype glob patterns
    /// By default the filetype is the extension of the file
    pub filetype_detect: Map<String, Vec<String>>,
}

impl EditorConfig {
    pub fn ignore_directories(&self) -> Vec<String> {
        self.ignore_directories.clone()
    }
}

pub(crate) struct MappingAsKeymap {
    events: Vec<KeyEvent>,
    actions: Vec<(Action, Option<ActionResult>)>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Mapping {
    pub(crate) key: String,
    pub(crate) actions: Vec<String>,
}

impl Mapping {
    pub fn to_keymap(&self) -> Option<MappingAsKeymap> {
        let keys = try_parse_keyevents(&self.key).ok()?;

        // try to find specified action
        let mut actions = vec![];

        for name in &self.actions {
            let mut name = name.as_str();
            let mut stop = None;
            if name.ends_with("!") {
                stop = Some(ActionResult::Ok);
                name = &name[..name.len() - 1];
            } else if name.ends_with("?") {
                stop = Some(ActionResult::Skipped);
                name = &name[..name.len() - 1];
            }

            // Try to parse goto_layer
            if let Some(goto) = parse_goto_layer(name) {
                let action = Action::Dynamic {
                    name: format!("Goto layer {}", goto),
                    fun: Arc::new(move |editor, id| {
                        change_keymap(editor, id, goto.clone());
                        ActionResult::Ok
                    }),
                    desc: String::new(),
                };

                actions.push((action, stop));
            } else if let Some(action) = find_by_name(name) {
                // Try to find action with name
                actions.push((action, stop));
            }
        }

        if actions.is_empty() {
            None
        } else {
            Some(MappingAsKeymap {
                events: keys,
                actions,
            })
        }
    }
}

fn parse_goto_layer(action: &str) -> Option<String> {
    let suffix = action.strip_prefix("goto_layer ")?;
    Some(suffix.to_string())
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct KeymapLayer {
    fallthrough: Option<String>,

    /// Whether to discard not found bindigs or insert them into the buffer
    discard: Option<bool>,

    /// On enter keymap actions
    on_enter: Option<Vec<String>>,

    /// On leave keymap actions
    on_leave: Option<Vec<String>>,

    /// Keymappings for this layer
    maps: Vec<Mapping>,
}

impl KeymapLayer {
    pub fn on_enter(&self, name: &str) -> Option<Action> {
        let mut actions = vec![];
        if let Some(acts) = &self.on_enter {
            for name in acts {
                match find_by_name(name) {
                    Some(action) => actions.push(action),
                    _ => log::error!("on enter: No such action {}", name),
                }
            }
        }

        if actions.is_empty() {
            return None;
        }

        let name = name.to_string();

        Action::Dynamic {
            name: format!("on_enter_{name}"),
            fun: Arc::new(move |editor, id| {
                let (win, _buf) = editor.win_buf(id);
                if win.keymap_layer != name {
                    return ActionResult::Skipped;
                }

                for action in &actions {
                    action.execute(editor, id);
                }

                ActionResult::Ok
            }),
            desc: String::new(),
        }
        .into()
    }

    pub fn on_leave(&self, name: &str) -> Option<Action> {
        let mut actions = vec![];
        if let Some(acts) = &self.on_leave {
            for name in acts {
                match find_by_name(name) {
                    Some(action) => actions.push(action),
                    _ => log::error!("on enter: No such action {}", name),
                }
            }
        }

        if actions.is_empty() {
            return None;
        }

        let name = name.to_string();

        Action::Dynamic {
            name: format!("on_leave_{name}"),
            fun: Arc::new(move |editor, id| {
                let (win, _buf) = editor.win_buf(id);
                if win.keymap_layer != name {
                    return ActionResult::Skipped;
                }

                for action in &actions {
                    action.execute(editor, id);
                }

                ActionResult::Ok
            }),
            desc: String::new(),
        }
        .into()
    }

    pub fn to_layer(&self, name: &str) -> Layer {
        let mut layer = Layer::new();
        layer.discard = self.discard.unwrap_or(false);
        layer.fallthrough = self.fallthrough.clone();

        for map in &self.maps {
            match map.to_keymap() {
                Some(MappingAsKeymap { events, actions }) => {
                    // Only a single action
                    if actions.len() == 1 {
                        let (action, _stop) = &actions[0];
                        layer.bind(&events, action);
                        continue;
                    }

                    // Multiple actions combined into one
                    let name = actions
                        .iter()
                        .map(|(action, _skip)| action.name())
                        .collect::<Vec<&str>>()
                        .join(",");
                    let action = Action::Dynamic {
                        name,
                        fun: Arc::new(move |editor, id| {
                            for (action, skip) in &actions {
                                let result = action.execute(editor, id);
                                let stop =
                                    skip.as_ref().map(|skip| &result > skip).unwrap_or(false);

                                if stop {
                                    break;
                                }
                            }

                            ActionResult::Ok
                        }),
                        desc: String::new(),
                    };

                    layer.bind(&events, &action);
                }
                _ => log::error!(
                    "Unknown keymapping: key: {}, actions: {:?}",
                    map.key,
                    map.actions
                ),
            }
        }

        layer.on_enter = self.on_enter(name);
        layer.on_leave = self.on_leave(name);

        layer
    }
}
