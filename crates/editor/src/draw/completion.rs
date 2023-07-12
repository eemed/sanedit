use std::cmp;

use sanedit_messages::redraw;

use crate::editor::windows::{Completion, Options};

pub(crate) fn draw_completion(
    completion: &Completion,
    options: &Options,
    scroll_offset: &mut usize,
) -> redraw::Completion {
    *scroll_offset = {
        let selected = completion.selected_pos().unwrap_or(0);
        if selected >= *scroll_offset + options.prompt_completions {
            // Make selected the bottom most completion, +1 to actually show
            // the selected completion
            selected - options.prompt_completions + 1
        } else {
            cmp::min(*scroll_offset, selected)
        }
    };
    let selected_relative_pos = completion.selected_pos().map(|pos| pos - *scroll_offset);
    let options = completion.matches_window(options.prompt_completions, *scroll_offset);
    redraw::Completion {
        options: options.into_iter().map(String::from).collect(),
        selected: selected_relative_pos,
    }
}
