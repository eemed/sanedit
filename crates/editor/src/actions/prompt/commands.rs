use crate::actions::jobs::{MatchedOptions, MatcherMessage};
use crate::actions::*;
use crate::common::matcher::MatchOption;
use crate::editor::windows::{Focus, SelectorOption};
use sanedit_messages::keyevents_to_string;

#[rustfmt::skip]
pub(crate) const COMMANDS: &[Action] = &[
    editor::quit,
    editor::build_project,
    editor::run_project,
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

    prompt::open_file,
    prompt::open_buffer,
    prompt::shell_command,
    prompt::select_theme,
    prompt::goto_line,
    prompt::goto_percentage,
    prompt::change_working_dir,
    prompt::grep,

    search::forward,
    search::backward,
    search::next_match,
    search::prev_match,

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

    cursors::select_line,
    cursors::start_selection,
    cursors::new_next_line,
    cursors::new_prev_line,
    cursors::new_to_next_search_match,
    cursors::new_to_all_search_matches,
    cursors::swap_selection_dir,

    view::scroll_up,
    view::scroll_down,

    indent::indent_line,
    indent::dedent_line,

    text_objects::select_parens,
    text_objects::select_in_parens,
    text_objects::select_square,
    text_objects::select_in_square,
    text_objects::select_curly,
    text_objects::select_in_curly,
    text_objects::select_angle,
    text_objects::select_in_angle,

    window::reload,
    window::goto_prev_buffer,

    filetree::show,

    locations::show,

    lsp::start_lsp,
    lsp::hover,
    lsp::goto_definition,
    lsp::complete,
    lsp::references,
    lsp::code_action,
    lsp::rename,

    popup::test,
    popup::close,
];

pub(crate) fn command_palette(editor: &Editor, id: ClientId) -> Vec<MatchOption> {
    let cmds = COMMANDS.to_vec();
    // Display descriptions in command palette
    cmds.iter()
        .map(|action| {
            let (win, _buf) = editor.win_buf(id);
            let mut description = String::new();
            if let Some(bind) = editor.keymap().find_bound_key(action.name()) {
                description = keyevents_to_string(&bind);
            }
            let value: String = action.description().into();
            MatchOption::with_description(&value, &description)
        })
        .collect()
}

pub(crate) fn find_action(name: &str) -> Option<Action> {
    for cmd in COMMANDS {
        if cmd.description() == name {
            return Some(cmd.clone());
        }
    }

    None
}

pub(crate) fn matcher_result_handler(editor: &mut Editor, id: ClientId, msg: MatcherMessage) {
    use MatcherMessage::*;

    let draw = editor.draw_state(id);
    draw.no_redraw_window();

    let (win, _buf) = editor.win_buf_mut(id);
    match msg {
        Init(sender) => {
            win.prompt.set_on_input(move |editor, id, input| {
                let _ = sender.blocking_send(input.to_string());
            });
            win.prompt.clear_options();
        }
        Progress(opts) => match opts {
            MatchedOptions::Options { matched, clear_old } => {
                if clear_old {
                    win.prompt.clear_options();
                }
                win.focus = Focus::Prompt;
                let opts: Vec<SelectorOption> =
                    matched.into_iter().map(SelectorOption::from).collect();
                let (win, _buf) = editor.win_buf_mut(id);
                win.prompt.provide_options(opts.into());
            }
            _ => {}
        },
    }
}
