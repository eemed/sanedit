use sanedit_messages::redraw::{self, Component, Item, ItemKind};

use crate::editor::windows::{Location, Locations};

use super::DrawContext;

pub(crate) fn draw(locs: &Locations, ctx: &mut DrawContext) -> redraw::Redraw {
    let selected = locs.selection_index().unwrap_or(0);
    let mut items = vec![];

    for entry in locs.iter() {
        let (kind, name) = match entry.loc {
            Location::Group { expanded, name, .. } => {
                let kind = ItemKind::Group {
                    expanded: *expanded,
                };
                (kind, name.to_string())
            }
            Location::Item {
                name,
                line,
                column,
                highlights,
            } => {
                let line = line.map(|n| n.to_string()).unwrap_or("?".into());
                let name = format!("{}: {}", line, name);
                (ItemKind::Item, name)
            }
        };

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
