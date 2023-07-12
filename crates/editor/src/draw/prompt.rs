use std::cmp;

use sanedit_messages::redraw::{self, Redraw};

use crate::editor::windows::Prompt;

use super::DrawContext;

pub(crate) fn draw(prompt: &Prompt, ctx: &mut DrawContext) -> Redraw {
    let compl_count = ctx.win.options.completions;
    let offset = &mut ctx.state.compl_scroll_offset;
    *offset = {
        let selected = prompt.selected_pos().unwrap_or(0);
        if selected >= *offset + compl_count {
            // Make selected the bottom most completion, +1 to actually show
            // the selected completion
            selected - compl_count + 1
        } else {
            cmp::min(*offset, selected)
        }
    };

    let msg = &prompt.message;
    let input = prompt.input();
    let cursor = prompt.cursor();
    let selected_relative_pos = prompt.selected_pos().map(|pos| pos - *offset);
    let options = prompt.matches_window(compl_count, *offset);
    redraw::Prompt::new(msg, &input, cursor, options, selected_relative_pos).into()
}
