use sanedit_messages::redraw::{Cell, Style};

pub(crate) fn into_cells(string: &str) -> Vec<Cell> {
    string.chars().map(Cell::from).collect()
}

pub(crate) fn into_cells_with_style(string: &str, style: Style) -> Vec<Cell> {
    let mut cells = into_cells(string);
    cells.iter_mut().for_each(|cell| cell.style = style);
    cells
}

pub(crate) fn into_cells_with_theme_pad_with(
    string: &str,
    style: &Style,
    width: usize,
) -> Vec<Cell> {
    let mut cells = into_cells_with_theme_with(string, style);
    pad_line(&mut cells, *style, width);
    cells
}

pub(crate) fn into_cells_with_theme_with(string: &str, style: &Style) -> Vec<Cell> {
    let mut cells = into_cells(string);
    cells.iter_mut().for_each(|cell| cell.style = *style);
    cells
}

#[allow(dead_code)]
pub(crate) fn pad_line_left(cells: &mut Vec<Cell>, style: Style, width: usize) {
    let left = width.saturating_sub(cells.len());
    let mut line = Vec::with_capacity(width);

    for _ in 0..left {
        line.push(Cell::with_style(style));
    }

    let items = std::mem::take(cells);
    line.extend(items);

    *cells = line;
}

pub(crate) fn pad_line(cells: &mut Vec<Cell>, style: Style, width: usize) {
    while cells.len() < width {
        cells.push(Cell::with_style(style));
    }

    cells.truncate(width);
}

pub(crate) fn center_pad(message: Vec<Cell>, pad_style: Style, width: usize) -> Vec<Cell> {
    let pad = (width.saturating_sub(message.len())) / 2;
    let mut result = into_cells_with_style(&" ".repeat(pad), pad_style);
    result.extend(message);
    pad_line(&mut result, pad_style, width);
    result
}
