use sanedit_messages::redraw::{self, Component, Item, ItemKind};

use crate::editor::filetree::Filetree;

use super::DrawContext;

pub(crate) fn draw(tree: &Filetree, ctx: &mut DrawContext) -> redraw::Redraw {
    let selected = ctx.editor.win.ft_view.selection;
    let mut items = vec![];

    for entry in tree.iter() {
        let kind = if entry.node.is_dir() {
            ItemKind::Group {
                expanded: entry.node.is_dir_expanded(),
            }
        } else {
            ItemKind::Item
        };

        let name = entry.name.to_string_lossy().to_string();
        let item = Item {
            name,
            kind,
            level: entry.level,
            highlights: vec![],
        };
        items.push(item);
    }

    let items = redraw::Items { items, selected };
    redraw::Redraw::Filetree(Component::Open(items))
}
