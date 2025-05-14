use super::*;

macro_rules! make_keymap {
    ($($key:expr, $action:ident),+,) => {
        vec![
            $(
                Mapping { key: $key.into(), actions: vec![stringify!($action).into()] },
             )*
        ]
    }
}

impl Default for Config {
    fn default() -> Self {
        let mut keymaps: Map<String, KeymapLayer> = Default::default();
        keymaps.insert(KeymapKind::Window.as_ref().to_string(), default::window());
        keymaps.insert(KeymapKind::Search.as_ref().to_string(), default::search());
        keymaps.insert(
            KeymapKind::Locations.as_ref().to_string(),
            default::locations(),
        );
        keymaps.insert(
            KeymapKind::Filetree.as_ref().to_string(),
            default::filetree(),
        );
        keymaps.insert(KeymapKind::Prompt.as_ref().to_string(), default::prompt());
        keymaps.insert(
            KeymapKind::Completion.as_ref().to_string(),
            default::completion(),
        );

        Config {
            editor: Default::default(),
            window: Default::default(),
            keymaps,
            snippet: Default::default(),
        }
    }
}

impl EditorConfig {
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
             "toml", ["*/Cargo.lock"],
             "yaml", ["*.yml"],
             "markdown", ["*.md"],
             "make", ["*/Makefile"],
        );

        ftmap
    }
}

impl Default for EditorConfig {
    fn default() -> Self {
        EditorConfig {
            // big_file_threshold_bytes: 100 * 1024 * 1024, // 100MB
            big_file_threshold_bytes: 1024 * 1024, // 1MB
            ignore_directories: [".git", "target"].into_iter().map(String::from).collect(),
            shell: "/bin/bash".into(),
            eol: EndOfLine::default(),
            detect_eol: true,
            detect_indent: true,
            filetype_detect: Self::default_filetype_map(),
        }
    }
}

pub(crate) fn search() -> KeymapLayer {
    #[rustfmt::skip]
        let map = make_keymap!(
            "ctrl+q",      quit,

            "esc",          prompt_close,
            "backspace",    prompt_remove_grapheme_before_cursor,
            "left",         prompt_prev_grapheme,
            "right",        prompt_next_grapheme,
            "enter",        prompt_confirm,
            "up",           prompt_history_next,
            "down",         prompt_history_prev,

            "ctrl+r",        search_toggle_regex,
        );

    KeymapLayer {
        on_enter: None,
        on_leave: None,
        fallthrough: None,
        discard: None,
        maps: map,
    }
}

pub(crate) fn prompt() -> KeymapLayer {
    #[rustfmt::skip]
        let map = make_keymap!(
             "ctrl+q",    quit,

             "esc",       prompt_close,
             "backspace", prompt_remove_grapheme_before_cursor,
             "left",      prompt_prev_grapheme,
             "right",     prompt_next_grapheme,
             "tab",       prompt_next_completion,
             "btab",      prompt_prev_completion,
             "enter",     prompt_confirm,
             "up",        prompt_history_next,
             "down",      prompt_history_prev,
        );

    KeymapLayer {
        on_enter: None,
        on_leave: None,
        fallthrough: None,
        discard: None,
        maps: map,
    }
}

pub(crate) fn completion() -> KeymapLayer {
    #[rustfmt::skip]
        let compl = make_keymap!(
             "tab",    next_completion,
             "btab",   prev_completion,
             "enter",  confirm_completion,
             "esc",    abort_completion,
        );

    KeymapLayer {
        on_enter: None,
        on_leave: None,
        fallthrough: Some(KeymapKind::Window.as_ref().into()),
        discard: None,
        maps: compl,
    }
}

pub(crate) fn locations() -> KeymapLayer {
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
    KeymapLayer {
        on_enter: None,
        on_leave: None,
        fallthrough: None,
        discard: None,
        maps: map,
    }
}

pub(crate) fn filetree() -> KeymapLayer {
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

    KeymapLayer {
        on_enter: None,
        on_leave: None,
        fallthrough: None,
        discard: None,
        maps: map,
    }
}

pub(crate) fn window() -> KeymapLayer {
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

            "esc",       cancel,
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

            "alt+2",     show_filetree,
            "alt+3",     show_locations,
        );

    KeymapLayer {
        on_enter: None,
        on_leave: None,
        fallthrough: None,
        discard: None,
        maps: map,
    }
}
