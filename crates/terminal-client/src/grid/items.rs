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
    area.width = max(min(area.width / 6, 50), 40);

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
    area.height = min(win.height, 10);
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
