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
    pub keymap: KeymapConfig,
}

pub(crate) const PROJECT_CONFIG: &str = "sanedit-project.toml";

pub(crate) fn read_config(config_path: &Path, working_dir: &Path) -> anyhow::Result<Config> {
    let local = working_dir.join(PROJECT_CONFIG);
    let config = config::Config::builder()
        .add_source(config::File::from(config_path))
        .add_source(config::File::from(local))
        .build()?;

    let config = config.try_deserialize::<Config>()?;
    log::info!("kmap: {:?}", config.keymap);

    Ok(config)
}

pub(crate) fn serialize_default_configuration(path: &Path) -> anyhow::Result<()> {
    use std::io::Write;

    let config = Config::default();
    let mut doc = to_document(&config).unwrap().to_owned();

    let mut visitor = Formatter {
        state: VisitState::Config,
        first: true,
    };
    visitor.visit_document_mut(&mut doc);

    let default_config = doc.to_string();
    let mut file = std::fs::File::create_new(path)?;
    file.write_all(default_config.as_bytes())?;

    Ok(())
}

struct Formatter {
    state: VisitState,
    first: bool,
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
        }

        let doc = {
            let keyname = key.get();
            match self.state {
                VisitState::Config => Config::get_field_docs(keyname),
                VisitState::Editor => editor::EditorConfig::get_field_docs(keyname),
                VisitState::Window => windows::WindowConfig::get_field_docs(keyname),
                VisitState::File => buffers::BufferConfig::get_field_docs(keyname),
                VisitState::Irrelevant => {
                    Err(documented::Error::NoDocComments("irrelevant".into()))
                }
            }
        };

        // Add docstrings as comments
        if let Ok(doc) = doc {
            let top = if self.first { "" } else { "\n" };
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

        self.first = false;
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
    File,
    Irrelevant,
}

impl VisitState {
    pub fn descend(self, key: &str) -> Self {
        match (self, key) {
            (VisitState::Config, "editor") => VisitState::Editor,
            (VisitState::Config, "window") => VisitState::Window,
            (VisitState::Config, "file") => VisitState::File,
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
    ($keymap:ident, $($action:ident, $bind:expr),+,) => {
        $(
            $keymap.insert(stringify!($action).into(), $bind.into());
         )*
    }
}

#[derive(Debug, Serialize, Deserialize, DocumentedFields)]
pub(crate) struct KeymapConfig {
    window: FxHashMap<String, String>,
    // search: FxHashMap<String, String>,
    // prompt: FxHashMap<String, String>,
    // completion: FxHashMap<String, String>,
    // locations: FxHashMap<String, String>,
    // filetree: FxHashMap<String, String>,
}

impl Default for KeymapConfig {
    fn default() -> Self {
        KeymapConfig {
            window: Self::window(),
        }
    }
}

impl KeymapConfig {
    pub fn window() -> FxHashMap<String, String> {
        let mut map = FxHashMap::default();

        #[rustfmt::skip]
        make_keymap!(map,
            quit,                             "ctrl+q",
            copy,                             "ctrl+c",
            paste,                            "ctrl+v",
            cut,                              "ctrl+x",
            build_project,                    "f2",
            run_project,                      "f3",

            save,                             "ctrl+s",
            remove_grapheme_before_cursor,    "backspace",
            remove_grapheme_after_cursor,     "delete",
            undo,                             "ctrl+z",
            redo,                             "ctrl+r",
            insert_newline,                   "enter",
            insert_tab,                       "tab",
            backtab,                          "btab",
            remove_line_after_cursor,         "alt+k",

            prev_line,                        "up",
            next_line,                        "down",
            prev_grapheme,                    "left",
            next_grapheme,                    "right",
            end_of_buffer,                    "alt+b",
            start_of_buffer,                  "alt+B",
            end_of_line,                      "alt+l",
            first_char_of_line,               "alt+L",
            next_word_start,                  "alt+w",
            prev_word_start,                  "alt+W",
            next_word_end,                    "alt+e",
            prev_word_end,                    "alt+E",
            next_paragraph,                   "alt+p",
            prev_paragraph,                   "alt+P",
            goto_matching_pair,               "alt+m",

            scroll_down,                      "alt+s",
            scroll_up,                        "alt+S",

            shell_command,                    "alt+r",
            command_palette,                  "ctrl+p",
            open_file,                        "ctrl+o",
            grep,                             "alt+f",

            search_forward,                   "ctrl+f",
            search_backward,                  "ctrl+g",
            clear_search_matches,             "ctrl+h",
            next_search_match,                "alt+n",
            prev_search_match,                "alt+N",

            keep_only_primary,                "esc",
            new_cursor_to_next_line,          "alt+down",
            new_cursor_to_prev_line,          "alt+up",
            new_cursor_to_next_search_match,  "ctrl+d",
            new_cursor_to_all_search_matches, "ctrl+l",
            start_selection,                  "alt+v",

            reload_window,                    "f5",
            goto_prev_buffer,                 "alt+'",

            select_line,                      "alt+o l",
            select_in_curly,                  "alt+o c",
            select_all_curly,                 "alt+o C",
            select_in_parens,                 "alt+o b",
            select_all_parens,                "alt+o B",
            select_in_square,                 "alt+o r",
            select_all_square,                "alt+o R",
            select_in_angle,                  "alt+o a",
            select_all_angle,                 "alt+o A",

            select_word,                      "alt+o w",

            show_filetree,                    "alt+2",
            show_locations,                   "alt+3",
        );

        map
    }
}
