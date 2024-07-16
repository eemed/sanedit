use sanedit_messages::redraw::{self, Source};

use crate::editor::windows::{Prompt, Search};

use super::DrawContext;

pub(crate) fn draw(prompt: &Prompt, search: &Search, _ctx: &mut DrawContext) -> redraw::Prompt {
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

    redraw::Prompt {
        message: msg,
        input,
        cursor,
        options: vec![],
        selected: None,
        source: Source::Search,
        max_completions: 0,
    }
}
