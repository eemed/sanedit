use std::path::Path;

use documented::DocumentedFields;
use serde::{Deserialize, Serialize};
use toml_edit::{
    ser::to_document,
    visit_mut::{visit_table_like_kv_mut, VisitMut},
    Item, KeyMut,
};

use crate::editor;

use super::{buffers, windows};
use rustc_hash::FxHashMap;

use super::Map;

#[derive(Debug, Default, Serialize, Deserialize, DocumentedFields)]
pub(crate) struct Config {
    ///
    /// Sanedit configuration
    ///
    /// Configuration can be overridden using a sanedit-project.toml file in project
    /// root. The file should be in the same format as this one and
    /// configuration may be partially updated
    ///
    pub editor: editor::EditorConfig,
    pub window: windows::WindowConfig,
    pub buffer: buffers::BufferConfig,
    pub keymaps: KeymapsConfig,
}

pub(crate) const PROJECT_CONFIG: &str = "sanedit-project.toml";

pub(crate) fn read_config(config_path: &Path, working_dir: &Path) -> anyhow::Result<Config> {
    let local = working_dir.join(PROJECT_CONFIG);
    let config = config::Config::builder()
        .add_source(config::File::from(config_path))
        .add_source(config::File::from(local))
        .build()?;
    let config = config.try_deserialize::<Config>()?;
    Ok(config)
}

pub(crate) fn serialize_default_configuration(path: &Path) -> anyhow::Result<()> {
    use std::io::Write;

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
        } else if self.state == VisitState::Keymaps {
            if node.is_array() {
                if let Some(arr) = node.as_array_mut() {
                    arr.set_trailing_comma(false);

                    for val in arr.iter_mut() {
                        val.decor_mut().set_prefix("\n    ");
                    }

                    if let Some(last) = arr.iter_mut().last() {
                        last.decor_mut().set_suffix(",\n");
                    }
                }
            } else {
            }
        }

        let doc = {
            let keyname = key.get();
            match self.state {
                VisitState::Config => Config::get_field_docs(keyname),
                VisitState::Editor => editor::EditorConfig::get_field_docs(keyname),
                VisitState::Window => windows::WindowConfig::get_field_docs(keyname),
                VisitState::Buffer => buffers::BufferConfig::get_field_docs(keyname),
                _ => Err(documented::Error::NoDocComments("irrelevant".into())),
            }
        };

        // Add docstrings as comments
        if let Ok(doc) = doc {
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
    Buffer,
    Keymaps,
    Irrelevant,
}

impl VisitState {
    pub fn descend(self, key: &str) -> Self {
        match (self, key) {
            (VisitState::Config, "editor") => VisitState::Editor,
            (VisitState::Config, "window") => VisitState::Window,
            (VisitState::Config, "buffer") => VisitState::Buffer,
            (VisitState::Config, "keymaps") => VisitState::Keymaps,
            (VisitState::Keymaps, _) => VisitState::Keymaps,
            _ => VisitState::Irrelevant,
        }
    }
}

#[derive(Debug, Clone, Serialize, Default, Deserialize, DocumentedFields)]
#[serde(default)]
pub(crate) struct LSPConfig {
    /// Command to run LSP
    pub command: String,

    /// Arguments to pass onto LSP command
    pub args: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, DocumentedFields)]
#[serde(default)]
pub(crate) struct EditorConfig {
    /// Large file threshold in bytes
    pub big_file_threshold_bytes: u64,

    /// Directories to ignore, when opening files etc.
    pub ignore_directories: Vec<String>,

    /// Default shell command
    pub shell: String,

    /// Shell command to build current project
    pub build_command: String,

    /// Shell command to run current project
    pub run_command: String,

    /// Autodetect eol from file
    pub detect_eol: bool,

    /// Autodetect indentation from file
    pub detect_indent: bool,

    /// Filetype glob patterns
    /// By default the filetype is the extension of the file
    pub filetype: Map<String, Vec<String>>,

    pub language_server: Map<String, LSPConfig>,
}

impl Default for EditorConfig {
    fn default() -> Self {
        EditorConfig {
            // big_file_threshold_bytes: 100 * 1024 * 1024, // 100MB
            big_file_threshold_bytes: 1024 * 1024, // 1MB
            ignore_directories: [".git", "target"].into_iter().map(String::from).collect(),
            shell: "/bin/bash".into(),
            build_command: String::new(),
            run_command: String::new(),
            detect_eol: true,
            detect_indent: true,
            filetype: Self::default_filetype_map(),
            language_server: Self::default_language_server_map(),
        }
    }
}

impl EditorConfig {
    pub fn ignore_directories(&self) -> Vec<String> {
        let ignore = self.ignore_directories.clone();
        ignore
    }

    fn default_filetype_map() -> FxHashMap<String, Vec<String>> {
        macro_rules! map {
            ($keymap:ident, $($ft: expr, $patterns:expr),+,) => {
                $(
                    $keymap.insert($ft.into(), $patterns.into_iter().map(String::from).collect());
                 )*
            }
        }

        let mut ftmap = FxHashMap::default();

        #[rustfmt::skip]
        map!(ftmap,
             "rust", ["*.rs"],
             "toml", ["**/Cargo.lock"],
             "yaml", ["*.yml"],
             "markdown", ["*.md"],
        );

        ftmap
    }

    fn default_language_server_map() -> FxHashMap<String, LSPConfig> {
        macro_rules! map {
            ($keymap:ident, $($ft: expr, $command:expr, $args:expr),+,) => {
                $(
                    $keymap.insert($ft.into(), LSPConfig { command: $command.into(), args: $args.into() });
                 )*
            }
        }

        let mut langmap = FxHashMap::default();

        #[rustfmt::skip]
        map!(langmap,
             "rust", "rust-analyzer", [],
        );

        langmap
    }
}

macro_rules! make_keymap {
    ($keymap:ident, $($key:expr, $action:ident),+,) => {
        $(
            $keymap.push(Mapping { key: $key.into(), action: stringify!($action).into() });
         )*
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Mapping {
    pub(crate) key: String,
    pub(crate) action: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct KeymapsConfig {
    pub(crate) window: Vec<Mapping>,
    // search: FxHashMap<String, String>,
    // prompt: FxHashMap<String, String>,
    // completion: FxHashMap<String, String>,
    // locations: FxHashMap<String, String>,
    // filetree: FxHashMap<String, String>,
}

impl Default for KeymapsConfig {
    fn default() -> Self {
        KeymapsConfig {
            window: Self::window(),
        }
    }
}

impl KeymapsConfig {
    pub fn window() -> Vec<Mapping> {
        let mut maps = Vec::default();

        #[rustfmt::skip]
        make_keymap!(maps,
            "ctrl+q",    quit,
            "ctrl+c",    copy,
            "ctrl+v",    paste,
            "ctrl+x",    cut,
            "f2",        build_project,
            "f3",        run_project,

            "ctrl+s",    save,
            "backspace", remove_grapheme_before_cursor,
            "delete",    remove_grapheme_after_cursor,
            "ctrl+z",    undo,
            "ctrl+r",    redo,
            "enter",     insert_newline,
            "tab",       insert_tab,
            "btab",      backtab,
            "alt+k",     remove_to_end_of_line,

            "up",        prev_line,
            "down",      next_line,
            "left",      prev_grapheme,
            "right",     next_grapheme,
            "alt+b",     end_of_buffer,
            "alt+B",     start_of_buffer,
            "alt+l",     end_of_line,
            "alt+L",     first_char_of_line,
            "alt+w",     next_word_start,
            "alt+W",     prev_word_start,
            "alt+e",     next_word_end,
            "alt+E",     prev_word_end,
            "alt+p",     next_paragraph,
            "alt+P",     prev_paragraph,
            "alt+m",     goto_matching_pair,

            "alt+s",     scroll_down,
            "alt+S",     scroll_up,

            "alt+r",     shell_command,
            "ctrl+p",    command_palette,
            "ctrl+o",    open_file,
            "alt+f",     grep,

            "ctrl+f",    search_forward,
            "ctrl+g",    search_backward,
            "ctrl+h",    clear_search_matches,
            "alt+n",     next_search_match,
            "alt+N",     prev_search_match,

            "esc",       keep_only_primary,
            "alt+down",  new_cursor_to_next_line,
            "alt+up",    new_cursor_to_prev_line,
            "ctrl+d",    new_cursor_to_next_search_match,
            "ctrl+l",    new_cursor_to_all_search_matches,
            "alt+v",     start_selection,

            "f5",        reload_window,
            "alt+'",     goto_prev_buffer,

            "alt+o l",   select_line,
            "alt+o c",   select_in_curly,
            "alt+o C",   select_all_curly,
            "alt+o b",   select_in_parens,
            "alt+o B",   select_all_parens,
            "alt+o r",   select_in_square,
            "alt+o R",   select_all_square,
            "alt+o a",   select_in_angle,
            "alt+o A",   select_all_angle,

            "alt+o w",   select_word,

            "alt+2",     show_filetree,
            "alt+3",     show_locations,
        );

        maps
    }
}
