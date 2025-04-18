use sanedit_messages::redraw::{self, PopupComponent};

use super::DrawContext;

pub(crate) fn draw(ctx: &mut DrawContext) -> Option<redraw::Redraw> {
    let popup = ctx.editor.win.popup();
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
    let redraw: redraw::Redraw = (*popup).clone().into();
    redraw.into()
}
