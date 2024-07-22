use sanedit_messages::redraw::{self, Component, Item, ItemKind};

use crate::editor::windows::{Location, Locations};

use super::DrawContext;

pub(crate) fn draw(locs: &Locations, ctx: &mut DrawContext) -> redraw::Redraw {
    let selected = ctx.editor.win.ft_view.selection;
    let mut items = vec![];

    for entry in locs.iter() {
        let kind = if let Location::Group { expanded, .. } = entry.loc {
            ItemKind::Group {
                expanded: *expanded,
            }
        } else {
            ItemKind::Item
        };

        let name = entry.loc.name().to_string();
        let item = Item {
            name,
            kind,
            level: entry.level,
            highlights: vec![],
        };
        items.push(item);
    }

    let items = redraw::Items { items, selected };
    redraw::Redraw::Locations(Component::Open(items))
}
