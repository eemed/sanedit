use sanedit_messages::redraw::{self, prompt::Source, Component, Redraw};

use crate::editor::windows::Focus;

use super::DrawContext;

pub(crate) fn draw(ctx: &mut DrawContext) -> Option<redraw::Redraw> {
    if ctx.focus_changed_from(Focus::Search) {
        ctx.state.prompt_scroll_offset = 0;
        ctx.state.last_prompt = None;
        return Redraw::Prompt(Component::Close).into();
    }

    let in_focus = ctx.editor.win.focus == Focus::Search;

    if !in_focus {
        return None;
    }

    draw_impl(ctx).into()
}

fn draw_impl(ctx: &mut DrawContext) -> redraw::Redraw {
    let prompt = &ctx.editor.win.prompt;
    let search = &ctx.editor.win.search;
    let msg = {
        let prompt = prompt.message();
        let kind = search.kind.tag();
        if kind.is_empty() {
            prompt.to_string()
        } else {
            format!("{} ({})", prompt, kind)
        }
    };
    let input = prompt.input().into();
    let cursor = prompt.cursor();

    redraw::prompt::Prompt {
        message: msg,
        input,
        cursor,
        options: vec![],
        selected: None,
        source: Source::Search,
        max_completions: 0,
    }
    .into()
}
