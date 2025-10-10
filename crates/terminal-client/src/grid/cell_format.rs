use sanedit_messages::redraw::{Cell, Style};

pub(crate) fn into_cells(string: &str) -> Vec<Cell> {
    string
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .map(Cell::from)
        .collect()
}

pub(crate) fn into_cells_with_style(string: &str, style: Style) -> Vec<Cell> {
    let mut cells = into_cells(string);
    cells.iter_mut().for_each(|cell| cell.style = style);
    cells
}

pub(crate) fn pad_line(cells: &mut Vec<Cell>, style: Style, width: usize) {
    while cells.len() < width {
        cells.push(Cell::with_style(style));
    }

    cells.truncate(width);
}
