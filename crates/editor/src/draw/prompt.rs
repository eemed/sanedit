use std::cmp;

use sanedit_messages::redraw::{self, prompt::Source, Component, Redraw};

use crate::editor::windows::Focus;

use super::DrawContext;

pub(crate) fn draw(ctx: &mut DrawContext) -> Vec<redraw::Redraw> {
    let mut results: Vec<redraw::Redraw> = vec![];

    let reopened = ctx
        .state
        .last_prompt
        .as_ref()
        .map(|p| p != ctx.editor.win.prompt.message())
        .unwrap_or(false);

    if ctx.focus_changed_from(Focus::Prompt) || reopened {
        ctx.state.prompt_scroll_offset = 0;
        ctx.state.last_prompt = None;
        results.push(Redraw::Prompt(Component::Close));
    }

    let in_focus = ctx.editor.win.focus() == Focus::Prompt;

    if !in_focus {
        return results;
    }

    results.push(draw_impl(ctx));
    results
}

fn draw_impl(ctx: &mut DrawContext) -> redraw::Redraw {
    let prompt = &ctx.editor.win.prompt;
    let compl_count = ctx.editor.win.config.max_prompt_completions;
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
    let options: Vec<redraw::choice::Choice> = prompt
        .options_window(compl_count, *offset)
        .into_iter()
        .map(|choice| {
            let c = choice.choice();
            redraw::choice::Choice {
                text: c.text().to_string(),
                description: c.description().to_string(),
                matches: choice.matches().to_vec(),
            }
        })
        .collect();
    let source = if prompt.is_simple() {
        Source::Simple
    } else {
        Source::Prompt
    };

    redraw::prompt::Prompt {
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
