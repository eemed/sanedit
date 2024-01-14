use crate::actions::Action;
use crate::actions::*;

pub(crate) const COMMANDS: &[Action] = &[
    editor::quit,
    prompt::open_file,
    movement::next_paragraph,
    movement::prev_grapheme,
    search::forward,
    search::backward,
];

pub(crate) fn command_palette() -> Vec<String> {
    let mut cmds = COMMANDS.to_vec();
    cmds.iter_mut().map(|cmd| cmd.name().to_string()).collect()
}

pub(crate) fn get_action_by_name(name: &str) -> Option<Action> {
    for cmd in COMMANDS {
        if cmd.name() == name {
            return Some(cmd.clone());
        }
    }

    None
}
