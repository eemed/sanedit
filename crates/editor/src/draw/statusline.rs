use sanedit_messages::redraw::{Redraw, Statusline};

use super::DrawContext;

pub(crate) fn draw(ctx: &mut DrawContext) -> Redraw {
    let buf = ctx.buf;

    let mut line = format!("{} ", buf.name());
    if buf.is_modified() {
        line.push_str("*");
    }
    if buf.is_saving() {
        line.push_str("(s)");
    }

    Statusline { line }.into()
}
