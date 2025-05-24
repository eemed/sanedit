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
        keymaps.insert(Mode::Normal.as_ref().into(), default::normal());
        keymaps.insert(Mode::Insert.as_ref().into(), default::insert());
        keymaps.insert(Mode::Select.as_ref().into(), default::select());

        keymaps.insert(Focus::Search.as_ref().into(), default::search());
        keymaps.insert(Focus::Locations.as_ref().into(), default::locations());
        keymaps.insert(Focus::Filetree.as_ref().into(), default::filetree());
        keymaps.insert(Focus::Prompt.as_ref().into(), default::prompt());
        keymaps.insert(Focus::Completion.as_ref().into(), default::completion());

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
            copy_on_delete: true,
        }
    }
}

pub(crate) fn search() -> KeymapLayer {
    #[rustfmt::skip]
    let map = make_keymap!(
        "space q",      quit,

        "esc",          prompt_close,
        "backspace",    prompt_remove_grapheme_before_cursor,
        "left",         prompt_prev_grapheme,
        "right",        prompt_next_grapheme,
        "enter",        prompt_confirm,
        "up",           prompt_history_next,
        "down",         prompt_history_prev,
    );

    KeymapLayer {
        on_enter: None,
        on_leave: None,
        fallthrough: None,
        maps: map,
    }
}

pub(crate) fn prompt() -> KeymapLayer {
    #[rustfmt::skip]
    let map = make_keymap!(
            "space q",    quit,

            "esc",       prompt_close,
            "§",         prompt_close,
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
        maps: map,
    }
}

pub(crate) fn completion() -> KeymapLayer {
    #[rustfmt::skip]
    let compl = make_keymap!(
        "tab",    completion_next,
        "btab",   completion_prev,
        "enter",  completion_confirm,
        "esc",    completion_abort,
        "§",      completion_abort,
    );

    KeymapLayer {
        on_enter: None,
        on_leave: None,
        fallthrough: Some(Mode::Insert),
        maps: compl,
    }
}

pub(crate) fn locations() -> KeymapLayer {
    #[rustfmt::skip]
    let map = make_keymap!(
            "alt+up", focus_window,
            "alt+k",  focus_window,
            "esc",    close_locations,
            "alt+q",  close_locations,
            "space q",close_locations,
            "§",      close_locations,
            "enter",  goto_loc_entry,
            "up",     prev_loc_entry,
            "down",   next_loc_entry,
            "k",      prev_loc_entry,
            "j",      next_loc_entry,
            "btab",   prev_loc_entry,
            "tab",    next_loc_entry,

            "p",      select_loc_parent,
            "s",      toggle_all_expand_locs,
            "K",      keep_locations,
            "R",      reject_locations,
    );
    KeymapLayer {
        on_enter: None,
        on_leave: None,
        fallthrough: None,
        maps: map,
    }
}

pub(crate) fn filetree() -> KeymapLayer {
    #[rustfmt::skip]
        let map = make_keymap!(
             "space q",   close_filetree,
             "esc",       close_filetree,
             "§",         close_filetree,
             "alt+right", focus_window,
             "alt+l",     focus_window,
             "enter",     goto_ft_entry,
             "up",        prev_ft_entry,
             "down",      next_ft_entry,
             "j",         prev_ft_entry,
             "k",         next_ft_entry,
             "btab",      prev_ft_entry,
             "tab",       next_ft_entry,
             "c",         ft_new_file,
             "d",         ft_delete_file,
             "p",         select_ft_parent,
             "s",         ft_goto_current_file,
        );

    KeymapLayer {
        on_enter: None,
        on_leave: None,
        fallthrough: None,
        maps: map,
    }
}

pub(crate) fn normal() -> KeymapLayer {
    #[rustfmt::skip]
    let map = make_keymap!(
        "space q", quit,
        "space s", strip_trailing_whitespace,
        "alt+v", select_line,
        "ctrl+w", new_window,

        "ctrl+s", save,
        "ctrl+c", copy,
        "ctrl+v", paste,
        "ctrl+x", cut,
        "ctrl+d", scroll_down,
        "ctrl+u", scroll_up,

        "esc", cancel,
        "§", cancel,
        "y", copy,
        "Y", copy_to_eol,
        "p", paste,
        "i", insert_mode,
        "u", undo,
        "U", redo,
        "h", prev_grapheme_on_line,
        "g k", prev_line,
        "g j", next_line,
        "l", next_grapheme_on_line,
        "k", prev_visual_line,
        "j", next_visual_line,
        "ctrl+o", jump_prev,
        "tab", jump_next,
        "g ;", jump_prev_change,
        "g ,", jump_next_change,
        "z z", view_to_cursor_middle,
        "z t", view_to_cursor_top,
        "z b", view_to_cursor_bottom,
        "H", cursor_to_view_top,
        "M", cursor_to_view_middle,
        "L", cursor_to_view_bottom,
        "g c", toggle_comment_lines,
        "J", join_lines,
        "ctrl+p", open_file,
        "v", start_selection,
        "$", end_of_line,
        "^", start_of_line,
        "0", first_char_of_line,
        "w", next_word_start,
        "b", prev_word_start,
        "V", select_line,
        "a", insert_mode_after,
        "A", insert_mode_end_of_line,
        "I", insert_mode_first_char_of_line,
        "!", shell_command,
        ":", command_palette,
        "/", search_forward,
        "?", search_backward,
        "%", goto_matching_pair,
        "c", first_char_of_line,
        "d", remove_line,
        "D", remove_to_eol,
        "x", remove_grapheme_after_cursor,
        "G", end_of_buffer,
        "g g", start_of_buffer,
        "o", newline_below,
        "O", newline_above,
        ">", indent_line,
        "<", dedent_line,
        "f", find_next_char_on_line,
        "F", find_prev_char_on_line,
        ";", next_searched_char,
        ",", prev_searched_char,
        "n", next_search_match,
        "N", prev_search_match,
        "#", search_prev_word_under_cursor,
        "*", search_next_word_under_cursor,
        "&", align_cursor_columns,

        "right", next_grapheme_on_line,
        "left", prev_grapheme_on_line,

        "alt+down", new_cursor_to_next_line,
        "alt+up",   new_cursor_to_prev_line,
        "alt+d",    new_cursor_to_next_search_match,
        "alt+l",    new_cursor_to_all_search_matches,
        "space b",  open_buffer,
        "space g",  grep,

        "g l",     goto_line,
        "g d",     goto_definition,
        "g r",     references,
        "g R",     rename,
        "space a", code_action,
        "space f", format,
        "space e", show_diagnostics,
        "space d", diagnostics_to_locations,
        "K",       hover,

        "-",         show_filetree,
        "alt+q",     show_locations,
        "backspace", goto_prev_buffer,
        "alt+j",     focus_locations,
        "alt+h",     focus_filetree,

        "s s", select_pattern,

        "s l", select_line,
        "s c", select_curly,
        "s C", select_curly_incl,
        "s b", select_parens,
        "s B", select_parens_incl,
        "s r", select_square,
        "s R", select_square_incl,
        "s a", select_angle,
        "s A", select_angle_incl,
        "s \"", select_double,
        "s '", select_single,
        "s `", select_backtick,
        "s p", select_paragraph,
        "s w", select_word,

        "e", next_word_end,
        "E", prev_word_end,

        "f5", reload_window,
    );

    KeymapLayer {
        on_enter: Some(vec!["show_diagnostic_highlights".into()]),
        on_leave: None,
        fallthrough: None,
        maps: map,
    }
}

pub(crate) fn insert() -> KeymapLayer {
    #[rustfmt::skip]
    let map = make_keymap!(
        "esc", normal_mode,
        "§", normal_mode,
        "left", prev_grapheme,
        "up", prev_line,
        "down", next_line,
        "right", next_grapheme,
        "backspace", remove_grapheme_before_cursor,
        "delete", remove_grapheme_after_cursor,
        "enter", insert_newline,
        "tab", insert_tab,
        "btab", backtab,
    );

    KeymapLayer {
        on_leave: None,
        on_enter: Some(vec!["hide_diagnostic_highlights".into()]),
        fallthrough: None,
        maps: map,
    }
}

pub(crate) fn select() -> KeymapLayer {
    #[rustfmt::skip]
    let map = make_keymap!(
        "i", nop,
        "x", remove_cursor_selections,
        "d", remove_cursor_selections,
        "c", change_cursor_selections,
        "y", copy,
        "esc", normal_mode,
        "§", normal_mode,
        ".", swap_selection_dir,

        "g", nop,
        "g c", toggle_comment_lines,
        "I", cursors_to_lines_start,
        "A", cursors_to_lines_end,
        "s", select_pattern,

        "ctrl+x", cut,
        "ctrl+c", copy,
    );

    KeymapLayer {
        on_enter: None,
        on_leave: None,
        fallthrough: Some(Mode::Normal),
        maps: map,
    }
}

#[allow(dead_code)]
pub(crate) fn window() -> KeymapLayer {
    #[rustfmt::skip]
        let map = make_keymap!(
            "space q",    quit,
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
        );

    KeymapLayer {
        on_enter: None,
        on_leave: None,
        fallthrough: None,
        maps: map,
    }
}
