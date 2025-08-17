pub(crate) mod completion;
pub(crate) mod cursors;
pub(crate) mod editor;
pub(crate) mod filetree;
pub(crate) mod hooks;
pub(crate) mod indent;
pub(crate) mod jobs;
pub(crate) mod locations;
pub(crate) mod lsp;
pub(crate) mod movement;
pub(crate) mod popup;
pub(crate) mod prompt;
pub(crate) mod search;
pub(crate) mod shell;
pub(crate) mod snippets;
pub(crate) mod syntax;
pub(crate) mod text;
pub(crate) mod text_objects;
pub(crate) mod view;
pub(crate) mod window;

use std::{fmt, sync::Arc};

use crate::editor::Editor;
use sanedit_server::ClientId;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ActionResult {
    /// Action completed succesfully
    Ok,

    /// Do not execute any further actions, wait for next one
    Skipped,

    /// Action failed to complete
    Failed,
}

impl<T, E> From<Result<T, E>> for ActionResult {
    fn from(value: Result<T, E>) -> Self {
        if value.is_ok() {
            ActionResult::Ok
        } else {
            ActionResult::Failed
        }
    }
}

#[derive(Clone)]
pub(crate) enum Action {
    Static {
        name: &'static str,
        fun: fn(&mut Editor, ClientId) -> ActionResult,
        desc: &'static str,
    },
    Dynamic {
        name: String,
        fun: Arc<dyn Fn(&mut Editor, ClientId) -> ActionResult>,
        desc: String,
    },
}

impl Action {
    pub fn execute(&self, editor: &mut Editor, id: ClientId) -> ActionResult {
        match self {
            Action::Static { fun, .. } => (fun)(editor, id),
            Action::Dynamic { fun, .. } => (fun)(editor, id),
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Action::Static { name, .. } => name,
            Action::Dynamic { name, .. } => &name,
        }
    }

    pub fn description(&self) -> &str {
        match self {
            Action::Static { desc, .. } => desc,
            Action::Dynamic { desc, .. } => &desc,
        }
    }
}

impl fmt::Debug for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

pub(crate) fn find_by_description(name: &str) -> Option<Action> {
    for cmd in COMMANDS {
        if cmd.description() == name {
            return Some(cmd.clone());
        }
    }

    None
}

pub(crate) fn find_by_name(name: &str) -> Option<Action> {
    for cmd in COMMANDS {
        if cmd.name() == name {
            return Some(cmd.clone());
        }
    }

    None
}

// Define commands that can be used in specific contexts

#[rustfmt::skip]
pub(crate) const COMMANDS: &[Action] = &[

    prompt::prompt_history_next,
    prompt::prompt_history_prev,
    prompt::prompt_next_completion,
    prompt::prompt_prev_completion,
    prompt::prompt_remove_grapheme_before_cursor,
    prompt::prompt_next_grapheme,
    prompt::prompt_prev_grapheme,
    prompt::prompt_confirm,
    prompt::prompt_close,
    prompt::prompt_jump,
    prompt::kill_jobs,

    completion::completion_confirm,
    completion::completion_abort,
    completion::completion_next,
    completion::completion_prev,

    locations::show_locations,
    locations::close_locations,
    locations::clear_locations,
    locations::next_loc_entry,
    locations::prev_loc_entry,
    locations::goto_loc_entry,
    locations::select_loc_parent,
    locations::toggle_all_expand_locs,
    locations::keep_locations,
    locations::reject_locations,
    locations::goto_next_loc_item,
    locations::goto_prev_loc_item,
    locations::focus_locations,
    locations::loc_select_first,
    locations::loc_select_last,
    locations::loc_stop_job,

    filetree::close_filetree,
    filetree::next_ft_entry,
    filetree::prev_ft_entry,
    filetree::select_ft_parent,
    filetree::ft_delete_file,
    filetree::ft_new_file,
    filetree::ft_rename_file,
    filetree::goto_ft_entry,
    filetree::ft_goto_current_file,
    filetree::show_filetree,
    filetree::focus_filetree,
    filetree::set_root,
    filetree::ft_select_first,
    filetree::ft_select_last,
    filetree::buffer_rename_file,
    filetree::buffer_remove_file,
    filetree::buffer_create_file,

    editor::quit,
    editor::copy,
    editor::copy_to_eol,
    editor::paste,
    editor::cut,
    editor::open_config,
    editor::open_new_scratch_buffer,
    editor::nop,
    editor::build_project,
    editor::run_project,

    text::save,
    text::save_all,
    text::save_as,
    text::undo,
    text::redo,
    text::strip_trailing_whitespace,
    text::remove_to_end_of_line,
    text::remove_grapheme_after_cursor,
    text::remove_grapheme_before_cursor,
    text::insert_newline,
    text::insert_tab,
    text::backtab,
    text::newline_above,
    text::newline_below,
    text::align_cursor_columns,
    text::comment_lines,
    text::uncomment_lines,
    text::toggle_comment_lines,
    text::join_lines,
    text::insert_literal,
    text::remove_line,
    text::remove_to_eol,
    text::reload_file_from_disk,
    text::uppercase,
    text::lowercase,
    text::rotate_selections,
    text::rotate_selections_backwards,
    text::set_language,
    text::change_to_eol,
    text::change_line,

    prompt::open_file,
    prompt::open_buffer,
    prompt::shell_command,
    prompt::select_theme,
    prompt::goto_line,
    prompt::goto_percentage,
    prompt::change_working_dir,
    prompt::grep,
    prompt::command_palette,
    prompt::buffer_snapshots,
    prompt::show_keymaps,

    search::search_forward,
    search::search_backward,
    search::next_search_match,
    search::prev_search_match,
    search::clear_search_matches,
    search::search_next_word_under_cursor,
    search::search_prev_word_under_cursor,

    movement::start_of_buffer,
    movement::end_of_buffer,
    movement::next_word_start,
    movement::prev_word_start,
    movement::next_paragraph,
    movement::prev_paragraph,
    movement::next_word_end,
    movement::prev_word_end,
    movement::goto_matching_pair,
    movement::end_of_line,
    movement::start_of_line,
    movement::next_line,
    movement::prev_line,
    movement::next_grapheme,
    movement::prev_grapheme,
    movement::first_char_of_line,
    movement::prev_visual_line,
    movement::next_visual_line,
    movement::find_next_char_on_line,
    movement::find_prev_char_on_line,
    movement::next_searched_char,
    movement::prev_searched_char,
    movement::prev_grapheme_on_line,
    movement::next_grapheme_on_line,

    cursors::start_selection,
    cursors::stop_selection,
    cursors::new_cursor_to_next_line,
    cursors::new_cursor_to_prev_line,
    cursors::swap_selection_dir,
    cursors::make_next_cursor_primary,
    cursors::make_prev_cursor_primary,
    cursors::remove_primary_cursor,
    cursors::select_to_next_word,
    cursors::select_to_prev_word,
    cursors::keep_only_primary,
    cursors::remove_cursor_selections,
    cursors::change_cursor_selections,
    cursors::cursors_to_lines_start,
    cursors::cursors_to_lines_end,
    cursors::jump_next,
    cursors::jump_prev,
    cursors::jump_next_change,
    cursors::jump_prev_change,
    cursors::cursor_to_view_top,
    cursors::cursor_to_view_middle,
    cursors::cursor_to_view_bottom,
    cursors::cursor_sort,
    cursors::cursor_sort_rev,
    cursors::cursor_trim_whitespace,
    cursors::cursor_select_individual_lines,

    view::scroll_up,
    view::scroll_down,

    indent::indent_line,
    indent::dedent_line,

    text_objects::select_buffer,
    text_objects::select_line,
    text_objects::select_line_without_eol,
    text_objects::select_parens_incl,
    text_objects::select_parens,
    text_objects::select_square_incl,
    text_objects::select_square,
    text_objects::select_curly_incl,
    text_objects::select_curly,
    text_objects::select_angle_incl,
    text_objects::select_angle,
    text_objects::select_word,
    text_objects::select_paragraph,
    text_objects::select_double,
    text_objects::select_double_incl,
    text_objects::select_single,
    text_objects::select_single_incl,
    text_objects::select_backtick,
    text_objects::select_backtick_incl,
    text_objects::select_pattern,

    window::reload_window,
    window::goto_prev_buffer,
    window::focus_window,
    window::cancel,
    window::new_window_horizontal,
    window::new_window_vertical,
    window::status,
    window::view_to_cursor,
    window::view_to_cursor_top,
    window::view_to_cursor_middle,
    window::view_to_cursor_bottom,
    window::hide_diagnostic_highlights,
    window::show_diagnostic_highlights,
    window::insert_mode,
    window::normal_mode,
    window::select_mode,
    window::insert_mode_after,
    window::insert_mode_end_of_line,
    window::insert_mode_first_char_of_line,
    window::snake,

    completion::complete,

    lsp::start_lsp,
    lsp::hover,
    lsp::goto_definition,
    lsp::references,
    lsp::code_action,
    lsp::rename,
    lsp::show_diagnostics,
    lsp::stop_lsp,
    lsp::restart_lsp,
    lsp::format,
    lsp::pull_diagnostics,
    lsp::diagnostics_to_locations,
    lsp::next_diagnostic,
    lsp::prev_diagnostic,
    lsp::show_symbols,
    lsp::show_signature_help,

    popup::close,

    // syntax::parse_syntax,

    snippets::snippet_jump_next,
    snippets::insert_snippet,
];
