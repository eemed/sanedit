use std::mem::take;

use sanedit_messages::redraw::{self, prompt::Source, Component, Kind, Redraw};

use crate::editor::windows::Focus;

use super::{DrawContext, Hash};

pub(crate) fn draw(ctx: &mut DrawContext) -> Option<redraw::Redraw> {
    if ctx.focus_changed_from(Focus::Search) {
        ctx.state.prompt_scroll_offset = 0;
        ctx.state.last_prompt = None;
        return Redraw::Prompt(Component::Close).into();
    }

    let in_focus = ctx.editor.win.focus() == Focus::Search;
    if !in_focus {
        return None;
    }

    let mut prompt = draw_impl(ctx);
    let selected = take(&mut prompt.selected);
    let hash = Hash::new(&prompt);
    if ctx.state.last_prompt.as_ref() == Some(&hash) {
        return Some(redraw::Redraw::Selection(Kind::Prompt, selected));
    }

    ctx.state.last_prompt = Some(hash);
    prompt.selected = selected;
    Some(redraw::Redraw::Prompt(Component::Update(prompt)))
}

fn draw_impl(ctx: &mut DrawContext) -> redraw::prompt::Prompt {
    let prompt = &ctx.editor.win.prompt;
    let msg = prompt.message().into();
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
}
