use crate::actions::jobs::{MatchedOptions, MatcherMessage};
use crate::actions::*;
use crate::common::matcher::{Match, MatchOption};
use crate::editor::windows::SelectorOption;
use sanedit_messages::keyevents_to_string;

#[rustfmt::skip]
pub(crate) const COMMANDS: &[Action] = &[
    editor::quit,

    text::save,
    text::copy,
    text::paste,
    text::undo,
    text::redo,

    prompt::open_file,
    prompt::shell_command,

    search::forward,
    search::backward,

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

    view::scroll_up,
    view::scroll_down,

    text_objects::select_parens,
    text_objects::select_in_parens,
    text_objects::select_square,
    text_objects::select_in_square,
    text_objects::select_curly,
    text_objects::select_in_curly,
    text_objects::select_angle,
    text_objects::select_in_angle,
];

pub(crate) fn command_palette(editor: &Editor, id: ClientId) -> Vec<MatchOption> {
    let cmds = COMMANDS.to_vec();
    // Display descriptions in command palette
    cmds.iter()
        .map(|action| {
            let (win, _buf) = editor.win_buf(id);
            let mut description = String::new();
            if let Some(bind) = win.keymap().find_bound_key(action.name()) {
                description = keyevents_to_string(&bind);
            }
            let value = action.description().into();
            MatchOption { value, description }
        })
        .collect()
}

// pub(crate) fn format_match(editor: &Editor, id: ClientId, mat: Match) -> SelectorOption {
//     let mut opt = SelectorOption::from(mat);
//     if let Some(action) = find_action(opt.value()) {
//         let (win, _buf) = editor.win_buf(id);
//         if let Some(bind) = win.keymap().find_bound_key(action.name()) {
//             opt.description = keyevents_to_string(&bind);
//         }
//     }

//     opt
// }

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
            MatchedOptions::ClearAll => win.prompt.clear_options(),
            MatchedOptions::Options(opts) => {
                let opts: Vec<SelectorOption> =
                    opts.into_iter().map(SelectorOption::from).collect();
                let (win, _buf) = editor.win_buf_mut(id);
                win.prompt.provide_options(opts.into());
            }
            _ => {}
        },
    }
}
