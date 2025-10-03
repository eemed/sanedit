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

impl Config {
    pub fn default_keymap() -> Map<String, KeymapLayer> {
        let mut keymaps: Map<String, KeymapLayer> = Default::default();
        keymaps.insert(Mode::Normal.as_ref().into(), default::normal());
        keymaps.insert(Mode::Insert.as_ref().into(), default::insert());
        keymaps.insert(Mode::Select.as_ref().into(), default::select());

        keymaps.insert(Focus::Search.as_ref().into(), default::search());
        keymaps.insert(Focus::Locations.as_ref().into(), default::locations());
        keymaps.insert(Focus::Filetree.as_ref().into(), default::filetree());
        keymaps.insert(Focus::Prompt.as_ref().into(), default::prompt());
        keymaps.insert(Focus::Completion.as_ref().into(), default::completion());
        keymaps
    }
}

impl EditorConfig {
    pub(crate) fn default_language_map() -> Arc<Map<String, Detect>> {
        macro_rules! map {
            ($keymap:ident, $ft: expr, $exts:expr, [], []) => {
                $keymap.insert(
                    $ft.into(),
                    Detect::new(
                        $exts.into_iter().map(String::from).collect(),
                        vec![],
                        vec![],
                    ),
                );
            };
            ($keymap:ident, $ft: expr, [], $globs:expr, []) => {
                $keymap.insert(
                    $ft.into(),
                    Detect::new(
                        vec![],
                        $globs.into_iter().map(String::from).collect(),
                        vec![],
                    ),
                );
            };
            ($keymap:ident, $ft: expr, $exts:expr, [], $shebangs:expr) => {
                $keymap.insert(
                    $ft.into(),
                    Detect::new(
                        $exts.into_iter().map(String::from).collect(),
                        vec![],
                        $shebangs.into_iter().map(String::from).collect(),
                    ),
                );
            };
            ($keymap:ident, $ft: expr, $exts:expr, $globs:expr, $shebangs:expr) => {
                $keymap.insert(
                    $ft.into(),
                    Detect::new(
                        $exts.into_iter().map(String::from).collect(),
                        $globs.into_iter().map(String::from).collect(),
                        $shebangs.into_iter().map(String::from).collect(),
                    ),
                );
            };
        }

        let mut d = Map::default();

        // Using LSP language identifiers
        map!(d, "asciidoc", ["adoc"], [], []);
        map!(d, "css", ["css", "scss", "sass"], [], []);
        map!(
            d,
            "dockerfile",
            [],
            ["**/Dockerfile", "**/Dockerfile.*"],
            []
        );
        map!(d, "go", ["go"], [], []);
        map!(d, "gitcommit", [], ["**/.git/COMMIT_EDITMSG"], []);
        map!(d, "glsl", ["vert", "geom", "tesc", "tese", "comp"], [], []);
        map!(d, "javascript", ["js"], [], []);
        map!(d, "javascriptreact", ["jsx"], [], []);
        map!(d, "make", [], ["**/Makefile"], []);
        map!(d, "markdown", ["md"], [], []);
        map!(
            d,
            "python",
            ["py"],
            [],
            ["#!/usr/bin/env python3", "#!/usr/bin/env python"]
        );
        map!(d, "rust", ["rs"], [], []);
        map!(
            d,
            "shellscript",
            ["sh", "bash"],
            ["**/.bashrc"],
            ["#!/bin/bash", "#!/usr/bin/env bash", "#!/bin/sh"]
        );
        map!(d, "toml", [], ["**/Cargo.lock"], []);
        map!(d, "typescript", ["ts"], [], []);
        map!(d, "typescriptreact", ["tsx"], [], []);
        map!(d, "vue", ["vue"], [], []);
        map!(d, "xml", ["xml", "svg"], [], ["<?xml version=\"1.0\" encoding=\"UTF-8\"?>"]);
        map!(d, "yaml", ["yml"], [], []);

        Arc::new(d)
    }
}

impl Default for EditorConfig {
    fn default() -> Self {
        EditorConfig {
            big_file_threshold_bytes: 100 * 1024 * 1024, // 100MB
            ignore: ["**/.git", "**/target", "**/node_modules"]
                .into_iter()
                .map(String::from)
                .collect::<Vec<String>>(),
            detect_eol: true,
            detect_indent: true,
            language_detect: Self::default_language_map(),
            copy_on_delete: true,
            auto_reload_changed_or_removed_file: false,
        }
    }
}

pub(crate) fn search() -> KeymapLayer {
    #[rustfmt::skip]
    let map = make_keymap!(
        "esc",          prompt_close,
        "§",            prompt_close,
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
        no_default: None,
    }
}

pub(crate) fn prompt() -> KeymapLayer {
    #[rustfmt::skip]
    let map = make_keymap!(
            "f1",        show_keymaps,
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
        no_default: None,
    }
}

pub(crate) fn completion() -> KeymapLayer {
    #[rustfmt::skip]
    let compl = make_keymap!(
        "f1",     show_keymaps,
        "tab",    completion_next,
        "btab",   completion_prev,
        "up",     completion_prev,
        "down",   completion_next,
        "enter",  completion_confirm,
        "esc",    completion_abort,
        "§",      completion_abort,
    );

    KeymapLayer {
        on_enter: None,
        on_leave: None,
        fallthrough: Some(Mode::Insert),
        maps: compl,
        no_default: None,
    }
}

pub(crate) fn locations() -> KeymapLayer {
    #[rustfmt::skip]
    let map = make_keymap!(
            "f1",     show_keymaps,
            "alt+k",  focus_window,
            "alt+h",  focus_filetree,
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

            "s",      loc_stop_job,
            "p",      select_loc_parent,
            "t",      toggle_all_expand_locs,
            "r",      reject_locations,

            "/",      keep_locations,
            "?",      keep_locations,
            "g g",    loc_select_first,
            "G",      loc_select_last,
    );
    KeymapLayer {
        on_enter: None,
        on_leave: None,
        fallthrough: None,
        maps: map,
        no_default: None,
    }
}

pub(crate) fn filetree() -> KeymapLayer {
    #[rustfmt::skip]
        let map = make_keymap!(
             "f1",        show_keymaps,
             "space q",   close_filetree,
             "esc",       close_filetree,
             "§",         close_filetree,

             "alt+l",     focus_window,
             "alt+j",     focus_locations,

             "enter",     goto_ft_entry,
             "up",        prev_ft_entry,
             "down",      next_ft_entry,
             "k",         prev_ft_entry,
             "j",         next_ft_entry,
             "btab",      prev_ft_entry,
             "tab",       next_ft_entry,
             "c",         ft_new_file,
             "d",         ft_delete_file,
             "p",         select_ft_parent,
             "s",         ft_goto_current_file,
             "r",         ft_rename_file,
             "m",         ft_rename_file,
             "R",         set_root,
             "g g",       ft_select_first,
             "G",         ft_select_last,
        );

    KeymapLayer {
        on_enter: None,
        on_leave: None,
        fallthrough: None,
        maps: map,
        no_default: None,
    }
}

pub(crate) fn normal() -> KeymapLayer {
    #[rustfmt::skip]
    let map = make_keymap!(
        "f1",     show_keymaps,
        "ctrl+w", new_window_vertical,
        "alt+w",  new_window_horizontal,

        "-",     show_filetree,
        "alt+q", show_locations,
        "alt+h", focus_filetree,
        "alt+j", focus_locations,

        "ctrl+a", select_buffer,
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
        "0", start_of_line,
        "^", first_char_of_line,
        "w", next_word_start,
        "b", prev_word_start,
        "V", select_line,
        "a", insert_mode_after,
        "A", insert_mode_end_of_line,
        "I", insert_mode_first_char_of_line,
        ":", command_palette,
        "/", search_forward,
        "?", search_backward,
        "%", goto_matching_pair,
        "d", remove_line,
        "D", remove_to_eol,
        "x", remove_grapheme_after_cursor,
        "G", end_of_buffer,
        "g g", start_of_buffer,
        "g p", next_paragraph,
        "g P", prev_paragraph,
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
        "e", next_word_end,
        "E", prev_word_end,
        "c", change_line,
        "C", change_to_eol,

        "alt+n", make_next_cursor_primary,
        "alt+N", make_prev_cursor_primary,
        "alt+d", remove_primary_cursor,
        "alt+down", new_cursor_to_next_line,
        "alt+up",   new_cursor_to_prev_line,

        "g n",     goto_line,
        "g l",     goto_next_loc_item,
        "g L",     goto_prev_loc_item,
        "g d",     goto_definition,
        "g r",     references,
        "g e",     next_diagnostic,
        "g E",     prev_diagnostic,
        "K",       hover,

        "space r", rename,
        "space q", quit,
        "space s", strip_trailing_whitespace,
        "space b", open_buffer,
        "space g", grep,
        "space a", code_action,
        "space f", format,
        "space e", show_diagnostics,
        "space d", diagnostics_to_locations,

        "backspace", goto_prev_buffer,

        "s s", select_pattern,
        "s l", select_line_without_eol,
        "s L", select_line,
        "s c", select_curly,
        "s C", select_curly_incl,
        "s {", select_angle,
        "s }", select_angle_incl,
        "s b", select_parens,
        "s B", select_parens_incl,
        "s (", select_angle,
        "s )", select_angle_incl,
        "s r", select_square,
        "s R", select_square_incl,
        "s [", select_angle,
        "s ]", select_angle_incl,
        "s a", select_angle,
        "s A", select_angle_incl,
        "s <", select_angle,
        "s >", select_angle_incl,
        "s \"", select_double,
        "s '", select_single,
        "s `", select_backtick,
        "s p", select_paragraph,
        "s w", select_word,

        "f5", reload_window,
    );

    KeymapLayer {
        on_enter: Some(vec!["show_diagnostic_highlights".into()]),
        on_leave: None,
        fallthrough: None,
        maps: map,
        no_default: None,
    }
}

pub(crate) fn insert() -> KeymapLayer {
    #[rustfmt::skip]
    let map = make_keymap!(
        "f1",  show_keymaps,
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
        "alt+k", show_signature_help,
        "alt+j", snippet_jump_next,
    );

    KeymapLayer {
        on_leave: None,
        on_enter: Some(vec!["hide_diagnostic_highlights".into()]),
        fallthrough: None,
        maps: map,
        no_default: None,
    }
}

pub(crate) fn select() -> KeymapLayer {
    #[rustfmt::skip]
    let map = make_keymap!(
        "f1",  show_keymaps,
        "i", nop,
        "a", cursor_select_individual_lines,
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
        "u", lowercase,
        "U", uppercase,
        "r", rotate_selections,
        "R", rotate_selections_backwards,
        "-", cursor_trim_whitespace,
        "g s", cursor_sort,
        "g S", cursor_sort_rev,

        "alt+j", next_paragraph,
        "alt+k", prev_paragraph,
        "alt+l", end_of_line,
        "alt+h", start_of_line,

        "ctrl+x", cut,
        "ctrl+c", copy,
    );

    KeymapLayer {
        on_enter: None,
        on_leave: None,
        fallthrough: Some(Mode::Normal),
        maps: map,
        no_default: None,
    }
}
