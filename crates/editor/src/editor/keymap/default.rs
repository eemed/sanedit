use crate::{actions::*, editor::keymap::KeyTrie, map};

use super::{KeyMappings, Keymap};

pub(crate) struct DefaultKeyMappings;

impl KeyMappings for DefaultKeyMappings {
    fn window() -> Keymap {
        let mut map = Keymap {
            root: KeyTrie::default(),
        };

        #[rustfmt::skip]
        map!(map,
             "ctrl+q", editor::quit,
             "ctrl+s", text::save,
             "up", movement::prev_line,
             "down", movement::next_line,
             "left", movement::prev_grapheme,
             "right", movement::next_grapheme,
             "backspace", text::remove_grapheme_before_cursor,
             "delete", text::remove_grapheme_after_cursor,

             "ctrl+c", text::copy,
             "ctrl+v", text::paste,
             // "ctrl+x", Action::next_visual_line,

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

             "alt+s", view::scroll_down,
             "alt+S", view::scroll_up,

             "ctrl+o", prompt::open_file,
             "ctrl+f", search::forward,
             "ctrl+g", search::backward,
             "ctrl+h", search::clear_matches,

             "esc", cursors::remove_secondary,
             "alt+down", cursors::new_next_line,
             "alt+up", cursors::new_prev_line,
             "ctrl+d", cursors::new_to_next_search_match,
             "ctrl+l", cursors::new_to_all_search_matches,

             "alt+n", search::next_match,
             "alt+N", search::prev_match,
             "alt+m", movement::goto_matching_pair,

             "alt+k", completion::complete,

             "ctrl+z", text::undo,
             "ctrl+r", text::redo,
             "ctrl+b", cursors::start_selection,

             "alt+r", prompt::shell_command,
             "alt+x", cursors::select_line,

             "ctrl+x", text_objects::select_curly,
             "ctrl+p", prompt::command_palette,
        );

        map
    }

    fn prompt() -> Keymap {
        let mut map = Keymap {
            root: KeyTrie::default(),
        };

        #[rustfmt::skip]
        map!(map,
             "ctrl+c", prompt::close,
             "backspace", prompt::remove_grapheme_before_cursor,
             "left", prompt::prev_grapheme,
             "right", prompt::next_grapheme,
             "tab", prompt::next_completion,
             "btab", prompt::prev_completion,
             "enter", prompt::confirm,
             "up", prompt::history_prev,
             "down", prompt::history_next,
        );

        map
    }

    fn search() -> Keymap {
        let mut map = Keymap {
            root: KeyTrie::default(),
        };

        #[rustfmt::skip]
        map!(map,
             "ctrl+c", search::close,
             "backspace", search::remove_grapheme_before_cursor,
             "left", search::prev_grapheme,
             "right", search::next_grapheme,
             "enter", search::confirm,
             "ctrl+enter", search::confirm_all,
             "alt+enter", search::confirm_all,
             "up", search::history_prev,
             "down", search::history_next,

             // "ctrl+r", search::toggle_regex,
             // "ctrl+s", search::toggle_select,
        );

        map
    }

    fn completion() -> Keymap {
        let mut map = Keymap {
            root: KeyTrie::default(),
        };

        map
    }
}
