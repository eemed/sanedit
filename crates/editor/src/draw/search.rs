use sanedit_messages::redraw;

use crate::editor::windows::{Options, Search};

pub(crate) fn draw_search(search: &Search, options: &Options) -> redraw::Prompt {
    let prompt = search.prompt();
    let msg = prompt.message();
    let input = prompt.input();
    let cursor = prompt.cursor();
    redraw::Prompt::new(msg, input, cursor, vec![], None).into()
}
