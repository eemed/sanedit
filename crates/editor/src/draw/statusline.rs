use sanedit_messages::redraw::{self, Statusline};

use super::DrawContext;

pub(crate) fn draw(ctx: &mut DrawContext) -> redraw::Statusline {
    let DrawContext { win, buf, .. } = ctx;

    let mut left = format!("{} ", buf.name());
    if buf.is_modified() {
        left.push_str("*");
    }
    if buf.is_saving() {
        left.push_str("(s)");
    }

    let cursor = win.primary_cursor();
    let cpos = cursor.pos();
    let blen = buf.len();
    let right = format!(
        "{}% {cpos}/{blen}",
        (blen as f64 / cpos.max(1) as f64).floor()
    );

    Statusline { left, right }
}
