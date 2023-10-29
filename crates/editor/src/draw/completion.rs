use std::cmp;

use sanedit_messages::redraw;

use crate::editor::windows::Completion;

use super::DrawContext;

pub(crate) fn draw(completion: &Completion, ctx: &mut DrawContext) -> redraw::Completion {
    let compl_count = ctx.win.options.completions;
    let offset = &mut ctx.state.compl_scroll_offset;
    *offset = {
        let selected = completion.selected_pos().unwrap_or(0);
        if selected >= *offset + compl_count {
            // Make selected the bottom most completion, +1 to actually show
            // the selected completion
            selected - compl_count + 1
        } else {
            cmp::min(*offset, selected)
        }
    };
    let selected_relative_pos = completion.selected_pos().map(|pos| pos - *offset);
    let options = completion.matches_window(compl_count, *offset);

    redraw::Completion {
        point: completion.point,
        options: options.into_iter().map(String::from).collect(),
        selected: selected_relative_pos,
    }
}
