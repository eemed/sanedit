use sanedit_messages::redraw::{self, Point, Popup, PopupComponent};

use super::DrawContext;

pub(crate) fn draw(ctx: &mut DrawContext) -> Option<redraw::Redraw> {
    let popup = &ctx.editor.win.popup;
    let show_popup = popup.is_some();
    let close_popup = !show_popup && ctx.state.last_show_popup == Some(true);
    ctx.state.last_show_popup = Some(show_popup);

    if close_popup {
        return Some(redraw::Redraw::Popup(PopupComponent::Close));
    }

    if !show_popup {
        return None;
    }

    let popup = popup.as_ref().unwrap();
    let cursor = ctx.editor.win.cursors.primary().pos();
    let point = ctx
        .editor
        .win
        .view()
        .point_at_pos(cursor)
        .unwrap_or(Point::default());

    let redraw: redraw::Redraw = Popup {
        severity: popup.severity,
        point,
        lines: popup.message.split("\n").map(String::from).collect(),
    }
    .into();

    redraw.into()
}
