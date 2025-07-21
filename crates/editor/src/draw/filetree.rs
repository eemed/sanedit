use sanedit_messages::redraw::{
    self,
    items::{Item, ItemKind},
    Component,
};

use crate::editor::windows::Focus;

use super::DrawContext;

pub(crate) fn draw(ctx: &mut DrawContext) -> Option<redraw::Redraw> {
    let show_ft = ctx.editor.win.ft_view.show;
    let close_ft = !show_ft && ctx.state.last_show_ft == Some(true);
    ctx.state.last_show_ft = Some(show_ft);

    if close_ft {
        return Some(redraw::Redraw::Filetree(Component::Close));
    }

    if !show_ft {
        return None;
    }

    draw_impl(ctx).into()
}

fn draw_impl(ctx: &mut DrawContext) -> redraw::Redraw {
    let tree = ctx.editor.filetree;
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

        let name = entry.name_to_str_lossy().to_string();
        let item = Item {
            location: None,
            name,
            kind,
            level: entry.level(),
            highlights: vec![],
        };
        items.push(item);
    }

    let in_focus = ctx.editor.win.focus() == Focus::Filetree;
    let items = redraw::items::Items {
        items,
        selected,
        in_focus,
        is_loading: false,
    };
    redraw::Redraw::Filetree(Component::Open(items))
}
