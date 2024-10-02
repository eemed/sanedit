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
pub(crate) mod syntax;
pub(crate) mod text;
pub(crate) mod text_objects;
pub(crate) mod view;
pub(crate) mod window;

use std::fmt;

use crate::editor::Editor;
use sanedit_server::ClientId;

#[derive(Clone)]
pub(crate) struct Action {
    name: &'static str,
    fun: fn(&mut Editor, ClientId),
    desc: &'static str,
}

impl Action {
    pub fn execute(&self, editor: &mut Editor, id: ClientId) {
        (self.fun)(editor, id)
    }

    pub fn name(&self) -> &str {
        self.name
    }

    pub fn description(&self) -> &str {
        self.desc
    }
}

impl fmt::Debug for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

pub(crate) fn find_by_description(actions: &[Action], name: &str) -> Option<Action> {
    for cmd in actions {
        if cmd.description() == name {
            return Some(cmd.clone());
        }
    }

    None
}

pub(crate) fn find_by_name(actions: &[Action], name: &str) -> Option<Action> {
    for cmd in actions {
        if cmd.name() == name {
            return Some(cmd.clone());
        }
    }

    None
}

// Define commands that can be used in specific contexts

#[rustfmt::skip]
pub(crate) const GLOBAL_COMMANDS: &[Action] = &[
    editor::quit,
    editor::build_project,
    editor::run_project,

    window::focus_window,
    filetree::show_filetree,
    locations::show_locations,
];

#[rustfmt::skip]
pub(crate) const SEARCH_COMMANDS: &[Action] = &[
    search::toggle_search_regex,
];

#[rustfmt::skip]
pub(crate) const PROMPT_COMMANDS: &[Action] = &[
    prompt::prompt_history_next,
    prompt::prompt_history_prev,
    prompt::prompt_next_completion,
    prompt::prompt_prev_completion,
    prompt::prompt_remove_grapheme_before_cursor,
    prompt::prompt_next_grapheme,
    prompt::prompt_prev_grapheme,
    prompt::prompt_confirm,
    prompt::close_prompt,

];

#[rustfmt::skip]
pub(crate) const COMPLETION_COMMANDS: &[Action] = &[
    completion::confirm_completion,
    completion::abort_completion,
    completion::next_completion,
    completion::prev_completion,
];

#[rustfmt::skip]
pub(crate) const FILETREE_COMMANDS: &[Action] = &[
    filetree::close_filetree,
    filetree::next_ft_entry,
    filetree::prev_ft_entry,
    filetree::select_ft_parent,
    filetree::ft_delete_file,
    filetree::ft_new_file,
    filetree::goto_ft_entry,
];

#[rustfmt::skip]
pub(crate) const LOCATIONS_COMMANDS: &[Action] = &[
    locations::close_locations,
    locations::clear_locations,
    locations::next_loc_entry,
    locations::prev_loc_entry,
    locations::goto_loc_entry,
    locations::select_loc_parent,
    locations::toggle_all_expand_locs,
];

#[rustfmt::skip]
pub(crate) const WINDOW_COMMANDS: &[Action] = &[
    editor::copy,
    editor::paste,
    editor::cut,
    editor::open_config,
    editor::open_new_scratch_buffer,

    text::save,
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

    prompt::open_file,
    prompt::open_buffer,
    prompt::shell_command,
    prompt::select_theme,
    prompt::goto_line,
    prompt::goto_percentage,
    prompt::change_working_dir,
    prompt::grep,
    prompt::command_palette,

    search::search_forward,
    search::search_backward,
    search::next_search_match,
    search::prev_search_match,
    search::clear_search_matches,

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

    cursors::start_selection,
    cursors::new_cursor_to_next_line,
    cursors::new_cursor_to_prev_line,
    cursors::new_cursor_to_next_search_match,
    cursors::new_cursor_to_all_search_matches,
    cursors::swap_selection_dir,
    cursors::make_next_cursor_primary,
    cursors::make_prev_cursor_primary,
    cursors::remove_primary_cursor,
    cursors::keep_only_primary,
    cursors::select_to_next_word,
    cursors::select_to_prev_word,
    cursors::keep_only_primary,

    view::scroll_up,
    view::scroll_down,

    indent::indent_line,
    indent::dedent_line,

    text_objects::select_line,
    text_objects::select_all_parens,
    text_objects::select_in_parens,
    text_objects::select_all_square,
    text_objects::select_in_square,
    text_objects::select_all_curly,
    text_objects::select_in_curly,
    text_objects::select_all_angle,
    text_objects::select_in_angle,
    text_objects::select_word,

    window::reload_window,
    window::goto_prev_buffer,

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

    popup::close,

    syntax::parse_syntax,
];
