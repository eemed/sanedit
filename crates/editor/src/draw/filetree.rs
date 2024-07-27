use sanedit_messages::redraw::{self, Component, Item, ItemKind};

use crate::editor::{filetree::Filetree, windows::Focus};

use super::DrawContext;

pub(crate) fn draw(ft: &Filetree, ctx: &mut DrawContext) -> Option<redraw::Redraw> {
    let show_ft = ctx.editor.win.ft_view.show;
    let close_ft = !show_ft && ctx.state.last_show_ft == Some(true);
    ctx.state.last_show_ft = Some(show_ft);

    if close_ft {
        return Some(redraw::Redraw::Filetree(Component::Close));
    }

    if !show_ft {
        return None;
    }

    draw_impl(ft, ctx).into()
}

fn draw_impl(tree: &Filetree, ctx: &mut DrawContext) -> redraw::Redraw {
    let selected = ctx.editor.win.ft_view.selection;
    let mut items = vec![];

    for entry in tree.iter() {
        let kind = if entry.node().is_dir() {
            ItemKind::Group {
                expanded: entry.node().is_dir_expanded(),
            }
        } else {
            ItemKind::Item
        };

        let name = entry.name().to_string_lossy().to_string();
        let item = Item {
            line: None,
            name,
            kind,
            level: entry.level(),
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
