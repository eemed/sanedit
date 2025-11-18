use std::{cmp, mem::take};

use sanedit_messages::redraw::{
    self,
    items::{ItemKind, ItemLocation, ItemsUpdate},
};
use sanedit_utils::either::Either;

use crate::editor::windows::Focus;

use super::{DrawContext, Hash};

pub(crate) fn draw(ctx: &mut DrawContext) -> Option<redraw::Redraw> {
    let show_loc = ctx.editor.win.locations.extra.show;
    let close_loc = !show_loc && ctx.state.last_loc.is_some();

    if close_loc {
        ctx.state.last_loc = None;
        ctx.state.last_loc_selection = None;
        ctx.state.loc_scroll_offset = 0;
        return Some(redraw::Redraw::Locations(ItemsUpdate::Close));
    }

    if !show_loc {
        ctx.state.last_loc = None;
        ctx.state.last_loc_selection = None;
        return None;
    }

    let mut locs = draw_impl(ctx);
    let selected = take(&mut locs.selected);
    let hash = Hash::new(&locs);

    if let Some(lhash) = ctx.state.last_loc.as_ref() {
        if lhash == &hash {
            if ctx.state.last_loc_selection == Some(selected) {
                return None;
            } else {
                ctx.state.last_loc_selection = Some(selected);
                locs.selected = selected;
                return Some(redraw::Redraw::Locations(ItemsUpdate::Selection(Some(selected))));
            }
        }
    }

    ctx.state.last_loc = Some(hash);
    ctx.state.last_loc_selection = Some(selected);
    locs.selected = selected;
    Some(redraw::Redraw::Locations(ItemsUpdate::Full(locs)))
}

fn draw_impl(ctx: &mut DrawContext) -> redraw::items::Items {
    let locs = &ctx.editor.win.locations;
    let max_locs = ctx.editor.win.config.max_locations;
    let offset = &mut ctx.state.loc_scroll_offset;
    *offset = {
        let selected = locs.selected_pos().unwrap_or(0);
        if selected >= *offset + max_locs {
            // Make selected the bottom most completion, +1 to actually show
            // the selected completion
            selected - max_locs + 1
        } else {
            cmp::min(*offset, selected)
        }
    };
    let vis_len = locs.visible_len();
    if *offset + max_locs > vis_len {
        let diff = *offset + max_locs - vis_len;
        *offset = offset.saturating_sub(diff);
    }
    let selected_relative_pos = locs.selected_pos().map(|pos| pos - *offset);

    let items: Vec<redraw::items::Item> = locs
        .iter()
        .skip(*offset)
        .take(max_locs)
        .map(|entry| match entry {
            Either::Left(group) => {
                let kind = ItemKind::Group {
                    expanded: group.is_expanded(),
                };
                let name = {
                    let gp = group.path();
                    let path = gp.strip_prefix(ctx.editor.working_dir).unwrap_or(gp);
                    path.to_string_lossy().to_string()
                };
                redraw::items::Item {
                    location: None,
                    name,
                    highlights: vec![],
                    kind,
                    level: 0,
                }
            }
            Either::Right(item) => {
                let location = item
                    .line()
                    .map(ItemLocation::Line)
                    .or_else(|| item.absolute_offset().map(ItemLocation::ByteOffset));

                redraw::items::Item {
                    location,
                    name: item.name().into(),
                    highlights: item.highlights().to_vec(),
                    kind: ItemKind::Item,
                    level: 1,
                }
            }
        })
        .collect();

    redraw::items::Items {
        title: locs.extra.title.clone(),
        items,
        selected: selected_relative_pos.unwrap_or(0),
        in_focus: ctx.editor.win.focus() == Focus::Locations,
        is_loading: locs.extra.is_loading,
    }
}
