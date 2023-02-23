use sanedit_messages::redraw::{self, Style, Theme, ThemeField};

use crate::{
    common::char::Replacement,
    editor::{
        buffers::Buffer,
        windows::{Cell, Cursors, View},
    },
};

pub(crate) fn draw_window(
    view: &View,
    cursors: &Cursors,
    buf: &Buffer,
    theme: &Theme,
) -> redraw::Window {
    let def = theme
        .get(ThemeField::Default.into())
        .unwrap_or(Style::default());
    let mut grid = vec![
        vec![
            redraw::Cell {
                text: " ".into(),
                style: def
            };
            view.width()
        ];
        view.height()
    ];

    for (line, row) in view.cells().iter().enumerate() {
        for (col, cell) in row.iter().enumerate() {
            if cell.width() == 0 {
                continue;
            }

            // Has to be char because it has width
            let ch = cell.char().unwrap();
            grid[line][col] = redraw::Cell {
                text: ch.display().into(),
                style: def,
            };

            for i in 1..cell.width() {
                grid[line][col + i] = redraw::Cell {
                    text: String::new(),
                    style: def,
                };
            }
        }
    }

    draw_end_of_buffer(&mut grid, view, theme);
    draw_trailing_whitespace(&mut grid, view, theme);

    let cursor = view
        .point_at_pos(cursors.primary().pos())
        .expect("cursor not at view");
    redraw::Window::new(grid, cursor)
}

fn draw_end_of_buffer(grid: &mut Vec<Vec<redraw::Cell>>, view: &View, theme: &Theme) {
    let def = theme
        .get(ThemeField::Default.into())
        .unwrap_or(Style::default());
    let eob = theme
        .get(ThemeField::EndOfBuffer.into())
        .unwrap_or(Style::default());
    let style = redraw::merge_cell_styles(&[def, eob]);
    for (line, row) in view.cells().iter().enumerate() {
        let is_empty = row
            .iter()
            .fold(true, |acc, cell| acc && matches!(cell, Cell::Empty));
        if is_empty {
            if let Some(rep) = view.options.replacements.get(&Replacement::BufferEnd) {
                grid[line][0] = redraw::Cell {
                    text: rep.as_str().into(),
                    style,
                };
            }
        }
    }
}

fn draw_trailing_whitespace(grid: &mut Vec<Vec<redraw::Cell>>, view: &View, theme: &Theme) {
    for (line, row) in view.cells().iter().enumerate() {}
}
