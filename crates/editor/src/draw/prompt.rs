use std::{cmp, mem::take};

use sanedit_messages::redraw::{self, prompt::Source, Component, Kind, Redraw};

use crate::editor::windows::Focus;

use super::{DrawContext, Hash};

pub(crate) fn draw(ctx: &mut DrawContext) -> Option<Redraw> {
    if ctx.focus_changed_from(Focus::Prompt) {
        ctx.state.prompt_scroll_offset = 0;
        ctx.state.last_prompt = None;
        ctx.state.last_prompt_selection = None;
        return Some(Redraw::Prompt(Component::Close));
    }

    let in_focus = ctx.editor.win.focus() == Focus::Prompt;
    if !in_focus {
        ctx.state.last_prompt = None;
        ctx.state.last_prompt_selection = None;
        return None;
    }

    let mut prompt = draw_impl(ctx);
    let selected = take(&mut prompt.selected);
    let hash = Hash::new(&prompt);
    if let Some(lhash) = ctx.state.last_prompt.as_ref() {
        if lhash == &hash {
            if ctx.state.last_prompt_selection == selected {
                return None;
            } else {
                ctx.state.last_prompt_selection = selected;
                prompt.selected = selected;
                return Some(redraw::Redraw::Selection(Kind::Prompt, selected));
            }
        }
    }

    ctx.state.last_prompt = Some(hash);
    ctx.state.last_prompt_selection = selected;
    prompt.selected = selected;
    Some(redraw::Redraw::Prompt(Component::Update(prompt)))
}

fn draw_impl(ctx: &mut DrawContext) -> redraw::prompt::Prompt {
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
        is_loading: prompt.is_options_loading(),
    }
}
