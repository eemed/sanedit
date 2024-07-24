use std::cmp;

use sanedit_messages::redraw::{self, CompletionOption, Component, Redraw};

use crate::editor::windows::{Completion, Focus};

use super::DrawContext;

pub(crate) fn draw(completion: &Completion, ctx: &mut DrawContext) -> Option<redraw::Redraw> {
    if ctx.focus_changed_from(Focus::Completion) {
        ctx.state.compl_scroll_offset = 0;
        return Redraw::Completion(Component::Close).into();
    }

    let in_focus = ctx.editor.win.focus == Focus::Completion;

    if !in_focus {
        return None;
    }

    draw_impl(completion, ctx).into()
}

fn draw_impl(completion: &Completion, ctx: &mut DrawContext) -> redraw::Redraw {
    let compl_count = ctx.editor.win.options.max_completions;
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
    let options: Vec<CompletionOption> = completion
        .options_window(compl_count, *offset)
        .into_iter()
        .map(|opt| CompletionOption {
            name: opt.value().into(),
            description: opt.description.clone(),
        })
        .collect();
    let match_len = completion
        .selector
        .options
        .get(0)
        .map(|mat| mat.matches().get(0).map(|o| o.len()))
        .flatten()
        .unwrap_or(0);

    redraw::Completion {
        point: completion.point,
        options,
        selected: selected_relative_pos,
        query_len: match_len,
    }
    .into()
}
