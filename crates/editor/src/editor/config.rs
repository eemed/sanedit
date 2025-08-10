mod default;
mod language;
mod project;

use std::{collections::VecDeque, path::Path, sync::Arc};

use language::ConfigSnippet;
use sanedit_messages::key::{try_parse_keyevents, KeyEvent};
use sanedit_server::ClientId;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use toml_edit::{
    ser::to_document,
    visit_mut::{visit_table_like_kv_mut, VisitMut},
    Item, KeyMut,
};

use crate::{
    actions::{find_by_name, Action, ActionResult},
    common::Choice,
    editor::{self},
};

use super::{
    buffers,
    keymap::{Layer, LayerKey},
    windows::{self, Focus, Mode},
    Editor, Map,
};

pub(crate) use language::{LSPConfig, LanguageConfig};
pub(crate) use project::*;

pub fn read_toml<T>(config_path: &Path) -> anyhow::Result<T>
where
    T: DeserializeOwned,
{
    use std::io::Read;
    let mut tomls = String::new();
    let mut toml = std::fs::File::open(config_path)?;
    toml.read_to_string(&mut tomls)?;
    let config = toml_edit::de::from_str::<T>(&tomls)?;
    Ok(config)
}

#[derive(Debug, Serialize, Deserialize, DocComment)]
#[serde(default)]
pub(crate) struct Config {
    #[serde(flatten)]
    pub editor: editor::EditorConfig,

    #[serde(flatten)]
    pub window: windows::WindowConfig,

    #[serde(flatten)]
    pub buffer: buffers::BufferConfig,

    pub keymaps: Map<String, KeymapLayer>,

    pub snippet: Vec<ConfigSnippet>,
}

impl Config {
    pub fn new(config_path: &Path, working_dir: &Path) -> Config {
        let mut config = match Self::try_new(config_path, working_dir) {
            Ok(config) => config,
            Err(e) => {
                log::warn!("Failed to load configuration, using default instead: {e}");
                Config::default()
            }
        };

        let kmaps = Config::default_keymap();
        for (name, mut kmap) in kmaps {
            if let Some(mut configured) = config.keymaps.remove(&name) {
                if configured.no_default.unwrap_or(false) {
                    continue;
                }

                // Override everything from default
                kmap.fallthrough = configured.fallthrough.or(kmap.fallthrough);
                kmap.on_enter = configured.on_enter.take().or(kmap.on_enter);
                kmap.on_leave = configured.on_leave.take().or(kmap.on_leave);
                for mapping in configured.maps {
                    kmap.maps.push(mapping);
                }
            }

            config.keymaps.insert(name, kmap);
        }

        // Extend default detect from user configuration
        let mut detect = EditorConfig::default_language_map();
        detect.extend(config.editor.language_detect);
        config.editor.language_detect = detect;

        config
    }

    fn try_new(config_path: &Path, _working_dir: &Path) -> anyhow::Result<Config> {
        read_toml::<Config>(config_path)
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
        for snip in &self.snippet {
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

    /// Glob patterns to ignore, when opening files etc.
    pub ignore: Vec<String>,

    /// Default shell command
    pub shell: String,

    /// Autodetect eol from file
    pub detect_eol: bool,

    /// Autodetect indentation from file
    pub detect_indent: bool,

    /// Language glob patterns
    /// By default the language is the extension of the file
    #[serde(skip_serializing)]
    pub language_detect: Map<String, Vec<String>>,

    /// Copy text to clipboard when deleting
    pub copy_on_delete: bool,
}

pub(crate) struct Keymapping {
    events: Vec<KeyEvent>,
    actions: VecDeque<MappedAction>,
}

#[derive(Clone)]
pub(crate) struct MappedAction {
    pub action: Action,
    /// Skip next actions if this action returns an result that is larger than
    /// this value.
    pub skip: Option<ActionResult>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Mapping {
    pub(crate) key: String,
    pub(crate) actions: Vec<String>,
}

impl Mapping {
    pub fn to_keymap(&self) -> Option<Keymapping> {
        let keys = try_parse_keyevents(&self.key).ok()?;

        // try to find specified action
        let mut actions = VecDeque::new();

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

            if let Some(action) = find_by_name(name) {
                // Try to find action with name
                actions.push_back(MappedAction { action, skip: stop });
            }
        }

        if actions.is_empty() {
            None
        } else {
            Some(Keymapping {
                events: keys,
                actions,
            })
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct KeymapLayer {
    /// Do not merge in default keymappings
    no_default: Option<bool>,

    fallthrough: Option<Mode>,

    /// On enter actions
    on_enter: Option<Vec<String>>,

    /// On leave actions
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
                if win.mode.as_ref() != name {
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
                if win.mode.as_ref() != name {
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
        layer.fallthrough = self.fallthrough.clone().map(|mode| LayerKey {
            focus: Focus::Window,
            mode,
        });

        for map in &self.maps {
            match map.to_keymap() {
                Some(Keymapping { events, actions }) => {
                    // Only a single action
                    if actions.len() == 1 {
                        let maction = &actions[0];
                        layer.bind(&events, &maction.action);
                        continue;
                    }

                    // Multiple actions combined into one
                    let name = actions
                        .iter()
                        .map(|maction| maction.action.name())
                        .collect::<Vec<&str>>()
                        .join(",");
                    let action = Action::Dynamic {
                        name,
                        fun: Arc::new(move |editor, id| {
                            run_mapped_action(editor, id, actions.clone())
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

/// Checks if we are prompting, and setups a new on_confirm that will run the
/// remaining callbacks, otherwise will continue to run actions
fn run_mapped_action2(
    editor: &mut Editor,
    id: ClientId,
    actions: VecDeque<MappedAction>,
    result: ActionResult,
    skip: Option<ActionResult>,
) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    let in_prompt = Focus::Prompt == win.focus();

    if in_prompt {
        let old = win.prompt.on_confirm();
        win.prompt.set_on_confirm(move |e, id, out| {
            let mut old_result = ActionResult::Ok;
            if let Some(old) = old {
                old_result = (old)(e, id, out);
            }

            run_mapped_action2(e, id, actions, old_result, skip)
        });
        return ActionResult::Ok;
    }

    let stop = skip.as_ref().map(|skip| &result > skip).unwrap_or(false);

    if stop {
        return result;
    }

    run_mapped_action(editor, id, actions)
}

/// Run mapped action.
///
/// If previous action was a promping one, use the result from the prompt
/// on_confirm callback instead and continue then.
///
/// Otherwise just run all the actions one by one
pub(crate) fn run_mapped_action(
    editor: &mut Editor,
    id: ClientId,
    mut actions: VecDeque<MappedAction>,
) -> ActionResult {
    while let Some(maction) = actions.pop_front() {
        let result = maction.action.execute(editor, id);
        return run_mapped_action2(editor, id, actions, result, maction.skip.clone());
    }

    ActionResult::Skipped
}
