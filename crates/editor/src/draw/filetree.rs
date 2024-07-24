use sanedit_messages::redraw::{self, Component, Item, ItemKind};

use crate::editor::{filetree::Filetree, windows::Focus};

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

    let in_focus = ctx.editor.win.focus == Focus::Filetree;
    let items = redraw::Items {
        items,
        selected,
        in_focus,
    };
    redraw::Redraw::Filetree(Component::Open(items))
}
