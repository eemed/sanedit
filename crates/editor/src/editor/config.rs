use std::path::Path;

use sanedit_messages::key::{try_parse_keyevents, KeyEvent};
use serde::{Deserialize, Serialize};
use toml_edit::{
    ser::to_document,
    visit_mut::{visit_table_like_kv_mut, VisitMut},
    Item, KeyMut,
};

use crate::{
    actions::{
        find_by_name, Action, COMPLETION_COMMANDS, FILETREE_COMMANDS, GLOBAL_COMMANDS,
        LOCATIONS_COMMANDS, PROMPT_COMMANDS, SEARCH_COMMANDS, WINDOW_COMMANDS,
    },
    editor,
};

use super::{buffers, keymap::KeymapKind, windows};
use rustc_hash::FxHashMap;

use super::Map;

#[derive(Debug, Default, Serialize, Deserialize, DocComment)]
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

pub(crate) fn read_config(config_path: &Path, working_dir: &Path) -> Config {
    match try_read_config(config_path, working_dir) {
        Ok(config) => config,
        Err(e) => {
            log::warn!("Failed to load configuration, using default instead: {e}");
            Config::default()
        }
    }
}

pub(crate) fn try_read_config(config_path: &Path, working_dir: &Path) -> anyhow::Result<Config> {
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
                VisitState::Config => Config::doc_comment(keyname),
                VisitState::Editor => editor::EditorConfig::doc_comment(keyname),
                VisitState::Window => windows::WindowConfig::doc_comment(keyname),
                VisitState::Buffer => buffers::BufferConfig::doc_comment(keyname),
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

#[derive(Debug, Clone, Serialize, Default, Deserialize, DocComment)]
#[serde(default)]
pub(crate) struct LSPConfig {
    /// Command to run LSP
    pub command: String,

    /// Arguments to pass onto LSP command
    pub args: Vec<String>,
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
        self.ignore_directories.clone()
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
    ($($key:expr, $action:ident),+,) => {
        vec![
            $(
                Mapping { key: $key.into(), action: stringify!($action).into() },
             )*
        ]
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Mapping {
    pub(crate) key: String,
    pub(crate) action: String,
}

impl Mapping {
    pub fn to_keymap(&self, kind: KeymapKind) -> Option<(Vec<KeyEvent>, Action)> {
        let mut actions: Vec<&[Action]> = match kind {
            KeymapKind::Search => [SEARCH_COMMANDS, PROMPT_COMMANDS].into(),
            KeymapKind::Prompt => [PROMPT_COMMANDS].into(),
            KeymapKind::Window => [WINDOW_COMMANDS].into(),
            KeymapKind::Completion => [WINDOW_COMMANDS, COMPLETION_COMMANDS].into(),
            KeymapKind::Filetree => [FILETREE_COMMANDS].into(),
            KeymapKind::Locations => [LOCATIONS_COMMANDS].into(),
        };

        actions.push(GLOBAL_COMMANDS);

        let keys = try_parse_keyevents(&self.key).ok()?;
        for list in actions {
            if let Some(action) = find_by_name(list, &self.action) {
                return Some((keys, action));
            }
        }

        None
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct KeymapsConfig {
    pub(crate) window: Vec<Mapping>,
    pub(crate) search: Vec<Mapping>,
    pub(crate) prompt: Vec<Mapping>,
    pub(crate) completion: Vec<Mapping>,
    pub(crate) locations: Vec<Mapping>,
    pub(crate) filetree: Vec<Mapping>,
}

impl Default for KeymapsConfig {
    fn default() -> Self {
        KeymapsConfig {
            window: Self::window(),
            search: Self::search(),
            prompt: Self::prompt(),
            completion: Self::completion(),
            locations: Self::locations(),
            filetree: Self::filetree(),
        }
    }
}

impl KeymapsConfig {
    pub fn search() -> Vec<Mapping> {
        #[rustfmt::skip]
        let map = make_keymap!(
            "ctrl+q",      quit,

            "esc",          close_prompt,
            "backspace",    prompt_remove_grapheme_before_cursor,
            "left",         prompt_prev_grapheme,
            "right",        prompt_next_grapheme,
            "enter",        prompt_confirm,
            "up",           prompt_history_next,
            "down",         prompt_history_prev,

            "ctrl+r",        toggle_search_regex,
        );
        map
    }

    pub fn prompt() -> Vec<Mapping> {
        #[rustfmt::skip]
        let map = make_keymap!(
             "ctrl+q",    quit,

             "esc",       close_prompt,
             "backspace", prompt_remove_grapheme_before_cursor,
             "left",      prompt_prev_grapheme,
             "right",     prompt_next_grapheme,
             "tab",       prompt_next_completion,
             "btab",      prompt_prev_completion,
             "enter",     prompt_confirm,
             "up",        prompt_history_next,
             "down",      prompt_history_prev,
        );
        map
    }

    pub fn completion() -> Vec<Mapping> {
        let mut map = Self::window();

        #[rustfmt::skip]
        let compl = make_keymap!(
             "tab",    next_completion,
             "btab",   prev_completion,
             "enter",  confirm_completion,
             "esc",    abort_completion,
        );

        map.extend(compl);
        map
    }

    pub fn locations() -> Vec<Mapping> {
        #[rustfmt::skip]
        let map = make_keymap!(
             "ctrl+q", quit,

             "alt+up", focus_window,
             "esc",    close_locations,
             "enter",  goto_loc_entry,
             "up",     prev_loc_entry,
             "down",   next_loc_entry,
             "btab",   prev_loc_entry,
             "tab",    next_loc_entry,
             "p",      select_loc_parent,
             "s",      toggle_all_expand_locs,
             "k",      keep_locations,
             "r",      reject_locations,

             "alt+1",  focus_window,
             "alt+2",  show_filetree,
             "alt+3",  close_locations,
        );
        map
    }

    pub fn filetree() -> Vec<Mapping> {
        #[rustfmt::skip]
        let map = make_keymap!(
             "ctrl+q", quit,

             "esc",       close_filetree,
             "alt+right", focus_window,
             "enter",     goto_ft_entry,
             "up",        prev_ft_entry,
             "down",      next_ft_entry,
             "btab",      prev_ft_entry,
             "tab",       next_ft_entry,
             "c",         ft_new_file,
             "d",         ft_delete_file,
             "p",         select_ft_parent,
             "s",         ft_goto_current_file,

             "alt+1",     focus_window,
             "alt+2",     close_filetree,
             "alt+3",     show_locations,
        );
        map
    }

    pub fn window() -> Vec<Mapping> {
        #[rustfmt::skip]
        let map = make_keymap!(
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

            "up",               prev_line,
            "down",             next_line,
            "ctrl+right",       next_word_end,
            "ctrl+left",        prev_word_start,
            "ctrl+shift+right", select_to_next_word,
            "ctrl+shift+left",  select_to_prev_word,

            "alt+U",     prev_line,
            "alt+u",     next_line,
            "left",      prev_grapheme,
            "right",     next_grapheme,
            // "alt+c",     next_grapheme,
            // "alt+C",     prev_grapheme,
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

            "esc",       prog_cancel,
            "alt+down",  new_cursor_to_next_line,
            "alt+up",    new_cursor_to_prev_line,
            "ctrl+d",    new_cursor_to_next_search_match,
            "ctrl+l",    new_cursor_to_all_search_matches,
            "alt+v",     start_selection,

            "f5",        reload_window,
            "alt+'",     goto_prev_buffer,

            "alt+o l",   select_line,
            "alt+o c",   select_curly,
            "alt+o C",   select_curly_incl,
            "alt+o b",   select_parens,
            "alt+o B",   select_parens_incl,
            "alt+o r",   select_square,
            "alt+o R",   select_square_incl,
            "alt+o a",   select_angle,
            "alt+o A",   select_angle_incl,
            "alt+o \"",  select_double,
            "alt+o '",   select_single,
            "alt+o `",   select_backtick,
            "alt+o p",   select_paragraph,
            "alt+o w",   select_word,

            "alt+x d",   goto_definition,
            "alt+x a",   code_action,
            "alt+x r",   references,
            "alt+x f",   format,
            "alt+x R",   rename,
            "alt+x h",   hover,

            "esc",        persist,
            "esc h",      prev_grapheme,
            "esc j",      next_line,
            "esc k",      prev_line,
            "esc l",      next_grapheme,
            "esc p",      next_paragraph,
            "esc P",      prev_paragraph,
            "esc w",      next_word_start,
            "esc W",      prev_word_start,
            "esc e",      next_word_end,
            "esc E",      prev_word_end,
            "esc /",      search_forward,
            "esc ?",      search_backward,
            "esc n",      next_search_match,
            "esc N",      prev_search_match,
            "esc !",      shell_command,
            "esc m",      goto_matching_pair,
            "esc V",      select_line,
            "esc v",      start_selection,
            "esc G",      end_of_buffer,
            "esc g",      start_of_buffer,
            "esc esc",    prog_cancel,
            "esc ctrl+p", command_palette,
            "esc ctrl+s", save,
            "esc ctrl+o", open_file,
            "esc ctrl+q", quit,
            "esc u",      undo,
            "esc U",      redo,
            "esc K",      hover,
            "esc tab",    insert_tab,
            "esc btab",   backtab,
            "esc o",      newline_below,
            "esc O",      newline_above,
            "esc D",      remove_to_end_of_line,

            "esc i",      clear_persist,
            "esc c",      clear_persist,

            "esc s l",   select_line,
            "esc s c",   select_curly,
            "esc s C",   select_curly_incl,
            "esc s b",   select_parens,
            "esc s B",   select_parens_incl,
            "esc s r",   select_square,
            "esc s R",   select_square_incl,
            "esc s a",   select_angle,
            "esc s A",   select_angle_incl,
            "esc s \"",  select_double,
            "esc s '",   select_single,
            "esc s `",   select_backtick,
            "esc s p",   select_paragraph,
            "esc s w",   select_word,

            "esc L d",   goto_definition,
            "esc L a",   code_action,
            "esc L r",   references,
            "esc L f",   format,
            "esc L R",   rename,
            "esc L h",   hover,
            "esc 0",     start_of_line,
            "esc A",     first_char_of_line,
            "esc a",     end_of_line,
            "esc :",     command_palette,

            "esc backspace", goto_prev_buffer,
            "esc ctrl+u",    scroll_up,
            "esc ctrl+d",    scroll_down,

            "alt+2",     show_filetree,
            "alt+3",     show_locations,
        );

        map
    }
}
