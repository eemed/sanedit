use sanedit_messages::redraw::{FileItem, FileItemKind, Filetree, Style};

use super::{
    ccell::{into_cells_with_style, CCell},
    item::GridItem,
    Rect,
};

pub(crate) fn open_filetree(win: Rect, ft: Filetree) -> GridItem<Filetree> {
    GridItem::new(ft, win)
}

pub(crate) fn format_item(item: &FileItem, name: Style, extra: Style) -> Vec<CCell> {
    let mut result = vec![];
    result.extend(into_cells_with_style(&"  ".repeat(item.level), extra));
    let fname = item.name.as_os_str().to_string_lossy();

    match item.kind {
        FileItemKind::Directory { expanded } => {
            if expanded {
                result.extend(into_cells_with_style("- ", extra));
            } else {
                result.extend(into_cells_with_style("+ ", extra));
            }
        }
        FileItemKind::File => {
            result.extend(into_cells_with_style("# ", extra));
        }
    }

    result.extend(into_cells_with_style(&fname, name));

    if matches!(item.kind, FileItemKind::Directory { .. }) {
        result.extend(into_cells_with_style("/", name));
    }

    result
}
