use sanedit_messages::redraw::{Item, ItemKind, Items, Style};

use super::{
    ccell::{into_cells_with_style, CCell},
    item::GridItem,
    Rect,
};

#[derive(Debug)]
pub(crate) struct CustomItems {
    pub(crate) items: Items,
    pub(crate) scroll: usize,
}

pub(crate) fn open_items(win: Rect, items: Items) -> GridItem<CustomItems> {
    GridItem::new(CustomItems { items, scroll: 0 }, win)
}

pub(crate) fn format_item(item: &Item, name: Style, extra: Style) -> Vec<CCell> {
    let mut result = vec![];
    result.extend(into_cells_with_style(&"  ".repeat(item.level), extra));

    match item.kind {
        ItemKind::Group { expanded } => {
            if expanded {
                result.extend(into_cells_with_style("- ", extra));
            } else {
                result.extend(into_cells_with_style("+ ", extra));
            }
        }
        ItemKind::Item => {
            result.extend(into_cells_with_style("# ", extra));
        }
    }

    result.extend(into_cells_with_style(&item.name, name));

    if matches!(item.kind, ItemKind::Group { .. }) {
        result.extend(into_cells_with_style("/", name));
    }

    result
}
