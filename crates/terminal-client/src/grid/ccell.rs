use std::ops::{Deref, DerefMut};

use sanedit_messages::redraw::{Cell, Size, Style};

#[derive(Debug, Clone)]
pub struct CCell {
    pub is_transparent: bool,
    pub cell: Cell,
}

impl CCell {
    pub fn transparent() -> CCell {
        CCell {
            is_transparent: true,
            cell: Cell::default(),
        }
    }

    pub fn from(ch: char) -> CCell {
        CCell {
            is_transparent: false,
            cell: Cell::from(ch),
        }
    }

    pub fn with_style(style: Style) -> CCell {
        CCell {
            is_transparent: false,
            cell: Cell::with_style(style),
        }
    }
}

impl Default for CCell {
    fn default() -> Self {
        CCell {
            is_transparent: false,
            cell: Cell::default(),
        }
    }
}

impl Deref for CCell {
    type Target = Cell;

    fn deref(&self) -> &Self::Target {
        &self.cell
    }
}

impl DerefMut for CCell {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.cell
    }
}

impl From<Cell> for CCell {
    fn from(value: Cell) -> Self {
        CCell {
            is_transparent: false,
            cell: value,
        }
    }
}

pub(crate) fn into_cells(string: &str) -> Vec<CCell> {
    string.chars().map(|ch| CCell::from(ch)).collect()
}

pub(crate) fn into_cells_with_style(string: &str, style: Style) -> Vec<CCell> {
    let mut cells = into_cells(string);
    cells.iter_mut().for_each(|cell| cell.style = style);
    cells
}

pub(crate) fn into_cells_with_style_pad(string: &str, style: Style, width: usize) -> Vec<CCell> {
    let mut cells = into_cells_with_style(string, style);
    pad_line(&mut cells, style, width);
    cells
}

pub(crate) fn into_cells_with_theme_pad_with(
    string: &str,
    style: &Style,
    width: usize,
) -> Vec<CCell> {
    let mut cells = into_cells_with_theme_with(string, style);
    pad_line(&mut cells, style.clone(), width);
    cells
}

pub(crate) fn into_cells_with_theme_with(string: &str, style: &Style) -> Vec<CCell> {
    let mut cells = into_cells(string);
    cells.iter_mut().for_each(|cell| cell.style = *style);
    cells
}

pub(crate) fn pad_line(cells: &mut Vec<CCell>, style: Style, width: usize) {
    while cells.len() < width {
        cells.push(CCell::with_style(style));
    }

    while cells.len() > width {
        cells.pop();
    }
}

pub(crate) fn size(cells: &mut [&mut [CCell]]) -> Size {
    let height = cells.len();
    let width = cells.get(0).map(|line| line.len()).unwrap_or(0);

    Size { width, height }
}

pub(crate) fn put_line(line: Vec<CCell>, pos: usize, target: &mut [&mut [CCell]]) {
    for (i, cell) in line.into_iter().enumerate() {
        target[pos][i] = cell;
    }
}

pub(crate) fn set_style(target: &mut [&mut [CCell]], style: Style) {
    for line in target.iter_mut() {
        for cell in line.iter_mut() {
            cell.style = style;
            cell.is_transparent = false;
        }
    }
}

pub(crate) fn center_pad(message: Vec<CCell>, pad_style: Style, width: usize) -> Vec<CCell> {
    let pad = (width.saturating_sub(message.len())) / 2;
    let mut result = into_cells_with_style(&" ".repeat(pad), pad_style);
    result.extend(message);
    pad_line(&mut result, pad_style, width);
    result
}

pub(crate) fn format_completion(
    left: &str,
    right: &str,
    mstyle: Style,
    dstyle: Style,
    width: usize,
    left_pad: bool,
) -> Vec<CCell> {
    let mut left = {
        let mut res = String::new();
        if left_pad {
            res.push(' ');
        }

        res.push_str(left);
        res.push(' ');
        res
    };

    let right = {
        let mut res = right.to_string();
        res.push(' ');
        res
    };

    // Fill space between
    let mut len = left.len() + right.len();
    while len < width {
        left.push(' ');
        len += 1;
    }

    let mut result = into_cells_with_style(&left, mstyle);
    result.extend(into_cells_with_style(&right, dstyle));
    result.truncate(width);
    result
}

pub(crate) fn format_option(
    left: &str,
    right: &str,
    mstyle: Style,
    dstyle: Style,
    width: usize,
) -> Vec<CCell> {
    let mut left = {
        let mut res = String::from(" ");
        res.push_str(&left);
        res.push(' ');
        res
    };

    let right = {
        let mut res = String::from("");
        res.push_str(&right);
        res.push(' ');
        res
    };

    // Fill space between
    let mut len = left.len() + right.len();
    while len < width {
        left.push(' ');
        len += 1;
    }

    let mut result = into_cells_with_style(&left, mstyle);
    result.extend(into_cells_with_style(&right, dstyle));
    result.truncate(width);
    result
}
