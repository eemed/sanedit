use sanedit_messages::redraw::{self, Component, ItemKind};
use sanedit_utils::either::Either;

use crate::editor::windows::{Focus, Group, Item, Locations};

use super::DrawContext;

pub(crate) fn draw(locs: &Locations, ctx: &mut DrawContext) -> Option<redraw::Redraw> {
    let show_loc = locs.show;
    let close_loc = !show_loc && ctx.state.last_show_loc == Some(true);
    ctx.state.last_show_loc = Some(show_loc);

    if close_loc {
        return Some(redraw::Redraw::Locations(Component::Close));
    }

    if !show_loc {
        return None;
    }

    draw_impl(locs, ctx).into()
}

fn draw_impl(locs: &Locations, ctx: &mut DrawContext) -> redraw::Redraw {
    let selected = locs.selection_index().unwrap_or(0);
    let mut items = vec![];

    for entry in locs.iter() {
        let (name, kind, hls, line) = match entry.loc {
            Either::Left(Group { expanded, path, .. }) => {
                let kind = ItemKind::Group {
                    expanded: *expanded,
                };
                let name = path.to_string_lossy().to_string();
                (name, kind, vec![], None)
            }
            Either::Right(Item {
                highlights,
                name,
                line,
                ..
            }) => (name.into(), ItemKind::Item, highlights.clone(), *line),
        };

        let item = redraw::Item {
            line,
            name: name.into(),
            kind,
            level: entry.level,
            highlights: hls,
        };
        items.push(item);
    }

    let in_focus = ctx.editor.win.focus == Focus::Locations;
    let items = redraw::Items {
        items,
        selected,
        in_focus,
    };
    redraw::Redraw::Locations(Component::Open(items))
}
