use std::{cmp, mem::take};

use sanedit_messages::redraw::{
    self,
    items::{ItemKind, ItemLocation, ItemsUpdate},
};


use super::{DrawContext, Hash};

pub(crate) fn draw(ctx: &mut DrawContext) -> Option<redraw::Redraw> {
    let show_snaps = ctx.editor.win.snapshot_view.show;
    let close_snaps = !show_snaps && ctx.state.last_snaps.is_some();

    if close_snaps {
        ctx.state.last_snaps = None;
        return Some(redraw::Redraw::Snapshots(ItemsUpdate::Close));
    }

    if !show_snaps {
        return None;
    }

    let mut items = draw_impl(ctx);
    let selected = take(&mut items.selected);
    let hash = Hash::new(&items);
    if ctx.state.last_snaps.as_ref() == Some(&hash) {
        return Some(redraw::Redraw::Snapshots(ItemsUpdate::Selection(Some(
            selected,
        ))));
    }

    ctx.state.last_snaps = Some(hash);
    items.selected = selected;
    Some(redraw::Redraw::Snapshots(ItemsUpdate::Full(items)))
}

fn draw_impl(ctx: &mut DrawContext) -> redraw::items::Items {
    let tree = ctx.editor.filetree;
    let selected = ctx.editor.win.snapshot_view.selection;
    let mut items = vec![];

    redraw::items::Items {
        title: todo!(),
        items,
        selected: todo!(),
        in_focus: todo!(),
        is_loading: todo!(),
    }
}
