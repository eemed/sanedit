use std::cmp::{max, min};

use sanedit_messages::redraw::{Item, ItemKind, Items, Style};

use super::{
    ccell::{into_cells_with_style, CCell},
    item::GridItem,
    Rect,
};

#[derive(Debug)]
pub(crate) enum Kind {
    Filetree,
    Locations,
}

#[derive(Debug)]
pub(crate) struct CustomItems {
    pub(crate) items: Items,
    pub(crate) scroll: usize,
    pub(crate) kind: Kind,
}

pub(crate) fn open_filetree(win: Rect, items: Items) -> GridItem<CustomItems> {
    let mut area = win;
    area.width = max(min(area.width / 6, 50), 30);

    if area.width > win.width {
        area.width = win.width;
    }

    GridItem::new(
        CustomItems {
            items,
            scroll: 0,
            kind: Kind::Filetree,
        },
        area,
    )
}

pub(crate) fn open_locations(win: Rect, items: Items) -> GridItem<CustomItems> {
    let mut area = win;
    let max = area.height + area.y;
    area.height /= 5;
    area.y = max - area.height;

    GridItem::new(
        CustomItems {
            items,
            scroll: 0,
            kind: Kind::Locations,
        },
        area,
    )
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
