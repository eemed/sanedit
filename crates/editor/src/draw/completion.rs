use std::cmp;

use sanedit_core::Choice;
use sanedit_messages::redraw::{self, Component, Redraw};

use crate::editor::windows::Focus;

use super::DrawContext;

pub(crate) fn draw(ctx: &mut DrawContext) -> Option<redraw::Redraw> {
    if ctx.focus_changed_from(Focus::Completion) {
        ctx.state.compl_scroll_offset = 0;
        return Redraw::Completion(Component::Close).into();
    }

    let in_focus = ctx.editor.win.focus == Focus::Completion;

    if !in_focus {
        return None;
    }

    draw_impl(ctx).into()
}

fn draw_impl(ctx: &mut DrawContext) -> redraw::Redraw {
    let completion = &ctx.editor.win.completion;
    let compl_count = ctx.editor.win.config.max_completions;
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
    let choices: Vec<Choice> = completion
        .choices_part(compl_count, *offset)
        .into_iter()
        .cloned()
        .collect();
    let query_len = ctx
        .editor
        .win
        .cursors()
        .primary()
        .pos()
        .saturating_sub(completion.started_at());

    redraw::completion::Completion {
        point: *completion.point(),
        choices,
        selected: selected_relative_pos,
        query_len: query_len as usize,
    }
    .into()
}
