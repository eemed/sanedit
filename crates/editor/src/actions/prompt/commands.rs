use crate::actions::*;
use crate::common::matcher::Match;
use crate::editor::windows::SelectorOption;
use sanedit_messages::keyevents_to_string;

#[rustfmt::skip]
pub(crate) const COMMANDS: &[Action] = &[
    editor::quit,

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
];

pub(crate) fn command_palette() -> Vec<String> {
    let cmds = COMMANDS.to_vec();
    // Display descriptions in command palette
    cmds.iter().map(|cmd| cmd.description().into()).collect()
}

pub(crate) fn format_match(editor: &Editor, id: ClientId, mat: Match) -> SelectorOption {
    let mut opt = SelectorOption::from(mat);
    if let Some(action) = find_action(opt.value()) {
        let (win, _buf) = editor.win_buf(id);
        if let Some(bind) = win.keymap().find_bound_key(action.name()) {
            opt.description = keyevents_to_string(&bind);
        }
    }

    opt
}

pub(crate) fn find_action(name: &str) -> Option<Action> {
    for cmd in COMMANDS {
        if cmd.description() == name {
            return Some(cmd.clone());
        }
    }

    None
}
