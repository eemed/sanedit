use std::{cmp, ffi::OsStr, ops::Range, path::PathBuf};

use sanedit_core::Choice;
use sanedit_messages::redraw::{self, Component, Redraw, Source};

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
        results.push(Redraw::Prompt(Component::Close).into());
    }

    let in_focus = ctx.editor.win.focus == Focus::Prompt;

    if !in_focus {
        return results;
    }

    results.push(draw_impl(ctx).into());
    results.into()
}

fn draw_impl(ctx: &mut DrawContext) -> redraw::Redraw {
    let prompt = &ctx.editor.win.prompt;
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
        .map(|choice| {
            if prompt.has_paths() {
                // Convert to path
                let os = unsafe { OsStr::from_encoded_bytes_unchecked(choice.value_raw()) };
                let path = PathBuf::from(os);
                // Strip working dir
                let path = path.strip_prefix(ctx.editor.working_dir).unwrap_or(&path);
                // Calculate how much we took off
                let off = choice.value_raw().len() - path.as_os_str().len();

                // Make matches
                let matches: Vec<Range<usize>> = choice
                    .matches()
                    .iter()
                    .cloned()
                    .map(|mut r| {
                        r.start -= off;
                        r.end -= off;
                        r
                    })
                    .collect();

                Choice::new(
                    &choice.value_raw()[off..],
                    matches,
                    choice.score(),
                    choice.description(),
                )
            } else {
                choice.clone()
            }
        })
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
