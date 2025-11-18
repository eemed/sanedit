use std::mem::take;

use sanedit_messages::redraw::{
    self,
    items::{Item, ItemKind, ItemsUpdate},
};

use crate::editor::windows::Focus;

use super::{DrawContext, Hash};

pub(crate) fn draw(ctx: &mut DrawContext) -> Option<redraw::Redraw> {
    let show_ft = ctx.editor.win.ft_view.show;
    let close_ft = !show_ft && ctx.state.last_ft.is_some();

    if close_ft {
        ctx.state.last_ft = None;
        return Some(redraw::Redraw::Filetree(ItemsUpdate::Close));
    }

    if !show_ft {
        return None;
    }

    // TODO optimize this draw
    let mut items = draw_impl(ctx);
    let selected = take(&mut items.selected);
    let hash = Hash::new(&items);
    if ctx.state.last_ft.as_ref() == Some(&hash) {
        return Some(redraw::Redraw::Filetree(ItemsUpdate::Selection(Some(
            selected,
        ))));
    }

    ctx.state.last_ft = Some(hash);
    items.selected = selected;
    Some(redraw::Redraw::Filetree(ItemsUpdate::Full(items)))
}

fn draw_impl(ctx: &mut DrawContext) -> redraw::items::Items {
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
    redraw::items::Items {
        title: String::new(),
        items,
        selected,
        in_focus,
        is_loading: false,
    }
}
