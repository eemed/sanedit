use std::cmp;

use sanedit_messages::redraw::{self, PromptType};

use crate::editor::windows::Prompt;

use super::DrawContext;

pub(crate) fn draw(prompt: &Prompt, ctx: &mut DrawContext) -> redraw::Prompt {
    let compl_count = ctx.win.options.completions;
    let offset = &mut ctx.state.prompt_scroll_offset;
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

    let msg = prompt.message().to_string();
    let input = prompt.input().into();
    let cursor = prompt.cursor();
    let selected_relative_pos = prompt.selected_pos().map(|pos| pos - *offset);
    let options = prompt
        .options_window(compl_count, *offset)
        .into_iter()
        .map(|m| m.into())
        .collect();

    redraw::Prompt {
        input,
        cursor,
        options,
        message: msg,
        selected: selected_relative_pos,
        ptype: PromptType::Overlay,
    }
}
