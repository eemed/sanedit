use sanedit_messages::redraw::{self, PromptType};

use crate::editor::windows::Search;

use super::DrawContext;

pub(crate) fn draw(search: &Search, _ctx: &mut DrawContext) -> redraw::Prompt {
    let prompt = &search.prompt;
    let msg = prompt.message().to_string();
    let input = prompt.input().into();
    let cursor = prompt.cursor();

    redraw::Prompt {
        message: msg,
        input,
        cursor,
        options: vec![],
        selected: None,
        ptype: PromptType::Oneline,
    }
}
