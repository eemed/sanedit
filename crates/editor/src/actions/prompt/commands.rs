use std::collections::HashMap;

use crate::actions::Action;
use crate::actions::*;
use crate::common::matcher::Match;
use crate::editor::windows::SelectorOption;
use lazy_static::lazy_static;

lazy_static! {
    static ref COMMANDS2: HashMap<u32, &'static str> = {
        let mut m = HashMap::new();
        m.insert(0, "foo");
        m.insert(1, "bar");
        m.insert(2, "baz");
        m
    };
}

pub(crate) const COMMANDS: &[Action] = &[
    editor::quit,
    prompt::open_file,
    movement::next_paragraph,
    movement::prev_grapheme,
    search::forward,
    search::backward,
];

pub(crate) fn command_palette() -> Vec<String> {
    let cmds = COMMANDS.to_vec();
    cmds.iter().map(|cmd| cmd.description().into()).collect()
}

pub(crate) fn format_match(editor: &Editor, id: ClientId, mat: Match) -> SelectorOption {
    let mut opt = SelectorOption::from(mat);
    if let Some(bound) = find_action(opt.value()) {
        // let (win, buf) = editor.win_buf(id);
        opt.description = format!("{:?}", id);
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

fn format_name(string: &str) -> String {
    let mut result = String::with_capacity(string.len());
    let mut first = true;
    for ch in string.chars() {
        if ch == '_' {
            result.push(' ');
            continue;
        }

        if first {
            result.push_str(&ch.to_uppercase().to_string());
            first = false;
        } else {
            result.push(ch);
        }
    }

    result
}
