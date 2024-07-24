use std::cmp;

use sanedit_messages::redraw::{self, Component, Redraw, Source};

use crate::editor::windows::{Focus, Prompt};

use super::DrawContext;

pub(crate) fn draw(prompt: &Prompt, ctx: &mut DrawContext) -> Option<redraw::Redraw> {
    if ctx.focus_changed_from(Focus::Prompt)
        || ctx
            .state
            .last_prompt
            .as_ref()
            .map(|p| p != ctx.editor.win.prompt.message())
            .unwrap_or(false)
    {
        ctx.state.prompt_scroll_offset = 0;
        ctx.state.last_prompt = None;
        return Redraw::Prompt(Component::Close).into();
    }

    let in_focus = ctx.editor.win.focus == Focus::Prompt;

    if !in_focus {
        return None;
    }

    draw_impl(prompt, ctx).into()
}

fn draw_impl(prompt: &Prompt, ctx: &mut DrawContext) -> redraw::Redraw {
    let compl_count = ctx.editor.win.options.max_prompt_completions;
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
    ctx.state.last_prompt = Some(prompt.message().to_string());

    let msg = prompt.message().to_string();
    let input = prompt.input().into();
    let cursor = prompt.cursor();
    let selected_relative_pos = prompt.selected_pos().map(|pos| pos - *offset);
    let options = prompt
        .options_window(compl_count, *offset)
        .into_iter()
        .map(|m| m.clone().into())
        .collect();
    let source = if prompt.is_simple() {
        Source::Simple
    } else {
        Source::Prompt
    };

    redraw::Prompt {
        input,
        cursor,
        options,
        message: msg,
        selected: selected_relative_pos,
        source,
        max_completions: compl_count,
    }
    .into()
}
