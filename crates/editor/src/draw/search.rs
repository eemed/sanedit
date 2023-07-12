use sanedit_messages::redraw::{self, Redraw};

use crate::editor::windows::Search;

use super::DrawContext;

pub(crate) fn draw(search: &Search, _ctx: &mut DrawContext) -> Redraw {
    let prompt = &search.prompt;
    let msg = &prompt.message;
    let input = prompt.input();
    let cursor = prompt.cursor();
    redraw::Prompt::new(msg, &input, cursor, vec![], None).into()
}
