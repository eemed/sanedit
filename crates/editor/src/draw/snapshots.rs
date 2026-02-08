use std::{mem::take, time::Instant};

use sanedit_messages::redraw::{
    self,
    snapshots::{SnapshotPoint, SnapshotsUpdate},
};

use crate::{common::human_readable_duration, editor::windows::Focus};

use super::{DrawContext, Hash};

pub(crate) fn draw(ctx: &mut DrawContext) -> Option<redraw::Redraw> {
    let show_snaps = ctx.editor.win.snapshot_view.show;
    let close_snaps = !show_snaps && ctx.state.last_snaps.is_some();

    if close_snaps {
        ctx.state.last_snaps = None;
        return Some(redraw::Redraw::Snapshots(SnapshotsUpdate::Close));
    }

    if !show_snaps {
        return None;
    }

    let mut items = draw_impl(ctx);
    let selected = take(&mut items.selected);
    let hash = Hash::new(&items);
    if ctx.state.last_snaps.as_ref() == Some(&hash) {
        return Some(redraw::Redraw::Snapshots(SnapshotsUpdate::Selection(Some(
            selected,
        ))));
    }

    ctx.state.last_snaps = Some(hash);
    items.selected = selected;
    Some(redraw::Redraw::Snapshots(SnapshotsUpdate::Full(items)))
}

fn draw_impl(ctx: &mut DrawContext) -> redraw::snapshots::Snapshots {
    let now = Instant::now();
    let snaps = ctx.editor.buf.snapshots();
    let points: Vec<SnapshotPoint> = snaps
        .iter()
        .map(|node| {
            let since = now.duration_since(node.timestamp);
            let ts = human_readable_duration(since);
            SnapshotPoint {
                title: ts.to_string(),
                next: node.next.clone(),
                id: node.id,
            }
        })
        .collect();

    let selected = ctx.editor.win.snapshot_view.selection;
    let in_focus = ctx.editor.win.focus() == Focus::Snapshots;

    redraw::snapshots::Snapshots {
        selected,
        in_focus,
        points,
    }
}
