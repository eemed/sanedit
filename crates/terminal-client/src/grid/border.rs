use sanedit_messages::redraw::{Cell, Style};

use super::ccell::{size, CCell};

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub(crate) enum Border {
    Box,
    Margin,
}

impl Border {
    pub fn top_left(&self) -> &str {
        match self {
            Border::Box => "┌",
            Border::Margin => " ",
        }
    }

    pub fn top_right(&self) -> &str {
        match self {
            Border::Box => "┐",
            Border::Margin => " ",
        }
    }

    pub fn bottom_right(&self) -> &str {
        match self {
            Border::Box => "┘",
            Border::Margin => " ",
        }
    }

    pub fn bottom_left(&self) -> &str {
        match self {
            Border::Box => "└",
            Border::Margin => " ",
        }
    }

    pub fn bottom(&self) -> &str {
        match self {
            Border::Box => "─",
            Border::Margin => " ",
        }
    }

    pub fn top(&self) -> &str {
        match self {
            Border::Box => "─",
            Border::Margin => " ",
        }
    }

    pub fn left(&self) -> &str {
        match self {
            Border::Box => "│",
            Border::Margin => " ",
        }
    }

    pub fn right(&self) -> &str {
        match self {
            Border::Box => "│",
            Border::Margin => " ",
        }
    }
}

pub(crate) fn draw_side_border_with_style<'a, 'b, F: Fn(usize, usize) -> Style>(
    border: Border,
    get_style: F,
    cells: &'a mut [&'b mut [CCell]],
) -> &'a mut [&'b mut [CCell]] {
    let size = size(cells);

    if size.width <= 2 && size.height <= 2 {
        return cells;
    }

    // Sides
    for (i, line) in cells.iter_mut().enumerate().skip(1) {
        line[0] = Cell {
            text: border.left().into(),
            style: get_style(i, 0),
        }
        .into();
        line[size.width - 1] = Cell {
            text: border.right().into(),
            style: get_style(i, size.width),
        }
        .into();
    }

    for l in cells.iter_mut() {
        let line = std::mem::take(l);
        let width = line.len();
        *l = &mut line[1..width - 1];
    }
    cells
}

fn draw_border_impl<'a, 'b, F: Fn(usize, usize) -> Style>(
    border: Border,
    get_style: F,
    mut cells: &'a mut [&'b mut [CCell]],
    strip: bool,
) -> &'a mut [&'b mut [CCell]] {
    let size = size(cells);

    if size.width <= 2 && size.height <= 2 {
        return cells;
    }

    // Top and bottom
    for i in 1..size.width {
        cells[0][i] = Cell {
            text: border.top().into(),
            style: get_style(0, i),
        }
        .into();
        cells[size.height - 1][i] = Cell {
            text: border.bottom().into(),
            style: get_style(size.height - 1, i),
        }
        .into();
    }

    // Sides
    for (i, line) in cells.iter_mut().enumerate().skip(1) {
        line[0] = Cell {
            text: border.left().into(),
            style: get_style(i, 0),
        }
        .into();
        line[size.width - 1] = Cell {
            text: border.right().into(),
            style: get_style(i, size.width),
        }
        .into();
    }

    // corners
    cells[0][0] = Cell {
        text: border.top_left().into(),
        style: get_style(0, 0),
    }
    .into();

    cells[size.height - 1][0] = Cell {
        text: border.bottom_left().into(),
        style: get_style(size.height - 1, 0),
    }
    .into();

    cells[0][size.width - 1] = Cell {
        text: border.top_right().into(),
        style: get_style(0, size.width - 1),
    }
    .into();

    cells[size.height - 1][size.width - 1] = Cell {
        text: border.bottom_right().into(),
        style: get_style(size.height - 1, size.width - 1),
    }
    .into();

    if strip {
        strip_border(cells);
    }
    cells
}

pub(crate) fn strip_border<'a, 'b>(
    mut cells: &'a mut [&'b mut [CCell]],
) -> &'a mut [&'b mut [CCell]] {
    let size = size(cells);

    cells = &mut cells[1..size.height - 1];
    for l in cells.iter_mut() {
        let line = std::mem::take(l);
        let width = line.len();
        *l = &mut line[1..width - 1];
    }

    cells
}

pub(crate) fn draw_border_no_strip<'a, 'b>(
    border: Border,
    style: Style,
    cells: &'a mut [&'b mut [CCell]],
) {
    draw_border_impl(border, |_, _| style, cells, false);
}

pub(crate) fn draw_border_with_style<'a, 'b, F: Fn(usize, usize) -> Style>(
    border: Border,
    get_style: F,
    cells: &'a mut [&'b mut [CCell]],
) -> &'a mut [&'b mut [CCell]] {
    draw_border_impl(border, get_style, cells, true)
}

/// Draw border and return inner cells to draw to
pub(crate) fn draw_border<'a, 'b>(
    border: Border,
    style: Style,
    cells: &'a mut [&'b mut [CCell]],
) -> &'a mut [&'b mut [CCell]] {
    draw_border_with_style(border, |_, _| style, cells)
}

#[allow(dead_code)]
pub(crate) fn draw_side_border<'a, 'b>(
    border: Border,
    style: Style,
    cells: &'a mut [&'b mut [CCell]],
) -> &'a mut [&'b mut [CCell]] {
    draw_side_border_with_style(border, |_, _| style, cells)
}
