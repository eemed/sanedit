use std::{cmp, mem::take};

use sanedit_messages::redraw::{self, completion::CompletionUpdate, Redraw};

use crate::editor::windows::Focus;

use super::{DrawContext, Hash};

pub(crate) fn draw(ctx: &mut DrawContext) -> Option<redraw::Redraw> {
    if ctx.focus_changed_from(Focus::Completion) {
        ctx.state.compl_scroll_offset = 0;
        ctx.state.last_compl = None;
        return Redraw::Completion(CompletionUpdate::Close).into();
    }

    let in_focus = ctx.editor.win.focus() == Focus::Completion;
    if !in_focus {
        ctx.state.last_compl = None;
        return None;
    }

    let mut compl = draw_impl(ctx);
    let selected = take(&mut compl.selected);
    let hash = Hash::new(&compl);
    if ctx.state.last_compl.as_ref() == Some(&hash) {
        return Some(redraw::Redraw::Completion(CompletionUpdate::Selection(selected)));
    }

    ctx.state.last_compl = Some(hash);
    compl.selected = selected;
    Some(redraw::Redraw::Completion(CompletionUpdate::Full(compl)))
}

fn draw_impl(ctx: &mut DrawContext) -> redraw::completion::Completion {
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
    let choices: Vec<redraw::choice::Choice> = completion
        .choices_part(compl_count, *offset)
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

    redraw::completion::Completion {
        point: *completion.point(),
        choices,
        selected: selected_relative_pos,
        item_offset_before_point: (completion.point_offset() - completion.item_start()) as usize,
    }
}
