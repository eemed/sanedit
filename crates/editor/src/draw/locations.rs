use sanedit_messages::redraw::{self, items::ItemKind, Component};
use sanedit_utils::either::Either;

use crate::editor::windows::Focus;

use super::DrawContext;

pub(crate) fn draw(ctx: &mut DrawContext) -> Option<redraw::Redraw> {
    let show_loc = ctx.editor.win.locations.show;
    let close_loc = !show_loc && ctx.state.last_show_loc == Some(true);
    ctx.state.last_show_loc = Some(show_loc);

    if close_loc {
        return Some(redraw::Redraw::Locations(Component::Close));
    }

    if !show_loc {
        return None;
    }

    draw_impl(ctx).into()
}

fn draw_impl(ctx: &mut DrawContext) -> redraw::Redraw {
    let locs = &ctx.editor.win.locations;
    let selected = locs.selection_index().unwrap_or(0);
    let mut items = vec![];

    for entry in locs.iter() {
        let (name, kind, hls, line, level) = match entry {
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
            Either::Right(item) => (
                item.name().into(),
                ItemKind::Item,
                item.highlights().to_vec(),
                item.line(),
                1,
            ),
        };

        let item = redraw::items::Item {
            line,
            name: name.into(),
            kind,
            level,
            highlights: hls,
        };
        items.push(item);
    }

    let in_focus = ctx.editor.win.focus == Focus::Locations;
    let items = redraw::items::Items {
        items,
        selected,
        in_focus,
    };
    redraw::Redraw::Locations(Component::Open(items))
}
