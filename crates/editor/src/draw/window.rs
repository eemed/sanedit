use std::ops::Range;

use sanedit_messages::redraw::{self, CursorStyle, Point, Style, Theme, ThemeField};
use sanedit_regex::Match;

use crate::{
    common::{char::Replacement, range::RangeUtils},
    editor::{
        buffers::Buffer,
        windows::{Cell, Cursor, Cursors, View, Window},
    },
};

pub(crate) fn draw_window(win: &Window, buf: &Buffer, theme: &Theme) -> redraw::Window {
    let style = theme.get(ThemeField::Default).unwrap_or(Style::default());
    let view = win.view();
    let mut grid = vec![vec![redraw::Cell::with_style(style); view.width()]; view.height()];

    for (line, row) in view.cells().iter().enumerate() {
        for (col, cell) in row.iter().enumerate() {
            if cell.width() == 0 {
                continue;
            }

            // Has to be char because it has width
            let ch = cell.char().unwrap();
            grid[line][col] = redraw::Cell {
                text: ch.display().into(),
                style,
            };

            for i in 1..cell.width() {
                grid[line][col + i] = redraw::Cell {
                    text: String::new(),
                    style,
                };
            }
        }
    }

    let cursors = win.cursors();
    let matches = &win.search.matches;

    draw_end_of_buffer(&mut grid, view, theme);
    draw_trailing_whitespace(&mut grid, view, theme);
    draw_search_highlights(&mut grid, matches, view, theme);
    draw_secondary_cursors(&mut grid, cursors, view, theme);
    let cursor = draw_primary_cursor(&mut grid, cursors.primary(), view, theme);
    redraw::Window::new(grid, cursor)
}

fn draw_search_highlights(
    grid: &mut Vec<Vec<redraw::Cell>>,
    matches: &[Match],
    view: &View,
    theme: &Theme,
) {
    let style = theme.get(ThemeField::Selection).unwrap_or(Style::default());

    let vrange = view.range();
    for m in matches {
        let mrange = m.range();
        if !vrange.overlaps(&mrange) {
            continue;
        }

        // TODO optimize
        let mut pos = vrange.start;
        for (line, row) in view.cells().iter().enumerate() {
            for (col, cell) in row.iter().enumerate() {
                if !matches!(cell, Cell::Empty) && mrange.contains(&pos) {
                    grid[line][col].style = style;
                }

                pos += cell.grapheme_len();
            }
        }
    }
}

fn draw_secondary_cursors(
    grid: &mut Vec<Vec<redraw::Cell>>,
    cursors: &Cursors,
    view: &View,
    theme: &Theme,
) {
    let style = theme.get(ThemeField::Selection).unwrap_or(Style::default());

    for cursor in cursors.cursors() {
        if !view.contains(cursor.pos()) {
            continue;
        }

        let area = cursor.selection().unwrap_or(cursor.pos()..cursor.pos() + 1);
        highlight_area(grid, area, view, style);
    }
}

fn highlight_area(
    grid: &mut Vec<Vec<redraw::Cell>>,
    area: Range<usize>,
    view: &View,
    hlstyle: Style,
) {
    let mut pos = view.range().start;

    for (line, row) in view.cells().iter().enumerate() {
        for (col, cell) in row.iter().enumerate() {
            if !matches!(cell, Cell::Empty) && area.contains(&pos) {
                grid[line][col].style = hlstyle;
            }

            pos += cell.grapheme_len();

            if area.end < pos {
                break;
            }
        }
    }
}

fn draw_primary_cursor(
    grid: &mut Vec<Vec<redraw::Cell>>,
    cursor: &Cursor,
    view: &View,
    theme: &Theme,
) -> redraw::Cursor {
    let style = theme.get(ThemeField::Selection).unwrap_or(Style::default());

    if let Some(area) = cursor.selection() {
        highlight_area(grid, area, view, style);
    }

    let has_selection = cursor.selection().is_some();
    let cstyle = if has_selection {
        CursorStyle::Line(false)
    } else {
        CursorStyle::Block(true)
    };
    let point = view
        .point_at_pos(cursor.pos())
        .expect("Primary cursor not in view");
    redraw::Cursor {
        style: cstyle,
        point,
    }
}

fn draw_end_of_buffer(grid: &mut Vec<Vec<redraw::Cell>>, view: &View, theme: &Theme) {
    let style = theme
        .get(ThemeField::EndOfBuffer)
        .unwrap_or(Style::default());
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
