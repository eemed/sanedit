use rustc_hash::FxHashMap;

use crate::{actions::*, editor::keymap::KeyTrie, map};

use super::{Keymap, KeymapKind};

pub(crate) struct DefaultKeyMappings;

impl DefaultKeyMappings {
    pub fn keymaps() -> FxHashMap<KeymapKind, Keymap> {
        use KeymapKind::*;

        let mut maps = FxHashMap::default();
        maps.insert(Window, Self::window());
        maps.insert(Prompt, Self::prompt());
        maps.insert(Search, Self::search());
        maps.insert(Completion, Self::completion());
        maps.insert(Filetree, Self::filetree());
        maps
    }

    fn window() -> Keymap {
        let mut map = Keymap {
            root: KeyTrie::default(),
        };

        #[rustfmt::skip]
        map!(map,
             "ctrl+q", editor::quit,
             "ctrl+c", editor::copy,
             "ctrl+v", editor::paste,
             "f2",     editor::build_project,
             "f1",     editor::run_project,

             "ctrl+s",    text::save,
             "backspace", text::remove_grapheme_before_cursor,
             "delete",    text::remove_grapheme_after_cursor,
             "ctrl+z",    text::undo,
             "ctrl+r",    text::redo,
             "enter",     text::insert_newline,
             "tab",       text::insert_tab,
             "btab",      text::backtab,

             // "ctrl+x", Action::next_visual_line,

             "up",    movement::prev_line,
             "down",  movement::next_line,
             "left",  movement::prev_grapheme,
             "right", movement::next_grapheme,
             "alt+b", movement::end_of_buffer,
             "alt+B", movement::start_of_buffer,
             "alt+l", movement::end_of_line,
             "alt+L", movement::start_of_line,
             // "alt+l", Action::next_visual_line,
             // "alt+L", Action::prev_visual_line,
             "alt+w", movement::next_word_start,
             "alt+W", movement::prev_word_start,
             "alt+e", movement::next_word_end,
             "alt+E", movement::prev_word_end,
             "alt+p", movement::next_paragraph,
             "alt+P", movement::prev_paragraph,
             "alt+m", movement::goto_matching_pair,

             "alt+s", view::scroll_down,
             "alt+S", view::scroll_up,

             "alt+r",  prompt::shell_command,
             "ctrl+p", prompt::command_palette,
             "ctrl+o", prompt::open_file,

             "ctrl+f", search::forward,
             "ctrl+g", search::backward,
             "ctrl+h", search::clear_matches,
             "alt+n",  search::next_match,
             "alt+N",  search::prev_match,

             "esc",      cursors::remove_secondary,
             "alt+down", cursors::new_next_line,
             "alt+up",   cursors::new_prev_line,
             "ctrl+d",   cursors::new_to_next_search_match,
             "ctrl+l",   cursors::new_to_all_search_matches,
             "alt+v",    cursors::start_selection,
             "alt+x",    cursors::select_line,

             "alt+k", completion::complete,

             "f5", window::reload,

             "alt+o c", text_objects::select_in_curly,
             "alt+o C", text_objects::select_curly,
             "alt+o b", text_objects::select_in_parens,
             "alt+o B", text_objects::select_parens,
             "alt+o r", text_objects::select_in_square,
             "alt+o R", text_objects::select_square,
             "alt+o a", text_objects::select_in_angle,
             "alt+o A", text_objects::select_angle,

        );

        map
    }

    fn prompt() -> Keymap {
        let mut map = Keymap {
            root: KeyTrie::default(),
        };

        #[rustfmt::skip]
        map!(map,
             "ctrl+q",      editor::quit,

             "ctrl+c",    prompt::close,
             "backspace", prompt::remove_grapheme_before_cursor,
             "left",      prompt::prev_grapheme,
             "right",     prompt::next_grapheme,
             "tab",       prompt::next_completion,
             "btab",      prompt::prev_completion,
             "enter",     prompt::confirm,
             "up",        prompt::history_prev,
             "down",      prompt::history_next,
        );

        map
    }

    fn search() -> Keymap {
        let mut map = Keymap {
            root: KeyTrie::default(),
        };

        #[rustfmt::skip]
        map!(map,
             "ctrl+q",      editor::quit,

            "ctrl+c",       prompt::close,
            "backspace",    prompt::remove_grapheme_before_cursor,
            "left",         prompt::prev_grapheme,
            "right",        prompt::next_grapheme,
            "enter",        prompt::confirm,
            "up",           prompt::history_prev,
            "down",         prompt::history_next,

            "ctrl+enter",   search::confirm_all,
            "alt+enter",    search::confirm_all,

             // "ctrl+s", search::toggle_select,
        );

        map
    }

    fn completion() -> Keymap {
        let mut map = DefaultKeyMappings::window();

        #[rustfmt::skip]
        map!(map,
             "tab",   completion::next,
             "btab",  completion::prev,
             "enter", completion::confirm,
             "esc",   completion::abort,

             // "ctrl+s", search::toggle_select,
        );

        map
    }

    fn filetree() -> Keymap {
        let mut map = Keymap {
            root: KeyTrie::default(),
        };

        #[rustfmt::skip]
        map!(map,
             "ctrl+q", editor::quit,

             "esc",   filetree::close_filetree,
             "enter", filetree::confirm,
             "up",    filetree::prev_entry,
             "down",  filetree::next_entry,
        );

        map
    }
}
