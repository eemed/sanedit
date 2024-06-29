use sanedit_messages::redraw::{self, FileItem, FileItemKind};

use crate::editor::filetree::Filetree;

use super::DrawContext;

pub(crate) fn draw(tree: &Filetree, ctx: &mut DrawContext) -> redraw::Redraw {
    let selected = ctx.editor.win.ft_view.selection;
    let mut items = vec![];

    for entry in tree.iter() {
        let kind = if entry.node.is_dir() {
            FileItemKind::Directory {
                expanded: entry.node.is_dir_expanded(),
            }
        } else {
            FileItemKind::File
        };

        let item = FileItem {
            name: entry.name,
            kind,
            level: entry.level,
        };
        items.push(item);
    }

    redraw::Filetree { items, selected }.into()
}
