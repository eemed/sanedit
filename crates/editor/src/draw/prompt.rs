use std::cmp;

use sanedit_messages::redraw;

use crate::editor::windows::{Prompt, Options};

pub(crate) fn draw_prompt(
    prompt: &Prompt,
    options: &Options,
    scroll_offset: &mut usize,
) -> redraw::Prompt {
    *scroll_offset = {
        let selected = prompt.selected_pos().unwrap_or(0);
        if selected >= *scroll_offset + options.prompt_completions {
            // Make selected the bottom most completion, +1 to actually show
            // the selected completion
            selected - options.prompt_completions + 1
        } else {
            cmp::min(*scroll_offset, selected)
        }
    };

    let msg = prompt.message();
    let input = prompt.input();
    let cursor = prompt.cursor();
    let selected_relative_pos = prompt.selected_pos().map(|pos| pos - *scroll_offset);
    let options = prompt.matches_window(options.prompt_completions, *scroll_offset);
    redraw::Prompt::new(msg, input, cursor, options, selected_relative_pos).into()
}
