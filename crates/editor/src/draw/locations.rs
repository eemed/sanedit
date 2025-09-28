use std::mem::take;

use sanedit_messages::redraw::{
    self,
    items::{ItemKind, ItemLocation},
    Component, Kind,
};
use sanedit_utils::either::Either;

use crate::editor::windows::Focus;

use super::{DrawContext, Hash};

pub(crate) fn draw(ctx: &mut DrawContext) -> Option<redraw::Redraw> {
    let show_loc = ctx.editor.win.locations.extra.show;
    let close_loc = !show_loc && ctx.state.last_loc.is_some();

    if close_loc {
        ctx.state.last_loc = None;
        return Some(redraw::Redraw::Locations(Component::Close));
    }
    if !show_loc {
        return None;
    }

    let mut items = draw_impl(ctx);
    let selected = take(&mut items.selected);
    let hash = Hash::new(&items);
    if ctx.state.last_loc.as_ref() == Some(&hash) {
        return Some(redraw::Redraw::Selection(Kind::Locations, Some(selected)));
    }

    ctx.state.last_loc = Some(hash);
    items.selected = selected;
    Some(redraw::Redraw::Locations(Component::Update(items)))
}

fn draw_impl(ctx: &mut DrawContext) -> redraw::items::Items {
    let locs = &ctx.editor.win.locations;
    let selected = locs.selection_index().unwrap_or(0);
    let mut items = vec![];

    for entry in locs.iter() {
        let (name, kind, hls, location, level) = match entry {
            Either::Left(group) => {
                let kind = ItemKind::Group {
                    expanded: group.is_expanded(),
                };
                let name = {
                    let gp = group.path();
                    let path = gp.strip_prefix(ctx.editor.working_dir).unwrap_or(gp);
                    path.to_string_lossy().to_string()
                };
                (name, kind, vec![], None, 0)
            }
            Either::Right(item) => {
                let location = item
                    .line()
                    .map(ItemLocation::Line)
                    .or_else(|| item.absolute_offset().map(ItemLocation::ByteOffset));
                (
                    item.name().into(),
                    ItemKind::Item,
                    item.highlights().to_vec(),
                    location,
                    1,
                )
            }
        };

        let item = redraw::items::Item {
            location,
            name,
            kind,
            level,
            highlights: hls,
        };
        items.push(item);
    }

    let in_focus = ctx.editor.win.focus() == Focus::Locations;
    redraw::items::Items {
        title: locs.extra.title.clone(),
        items,
        selected,
        in_focus,
        is_loading: locs.extra.is_loading,
    }
}
