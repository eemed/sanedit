use std::ops::Range;

use sanedit_messages::redraw::{self, CursorShape, Style, Theme, ThemeField};

use crate::{
    common::{char::Replacement, range::RangeUtils},
    editor::{
        buffers::Buffer,
        windows::{Cell, Cursor, Cursors, View, Window},
    },
};

pub(crate) fn draw_window(win: &Window, _buf: &Buffer, theme: &Theme) -> redraw::Window {
    let style = theme.get(ThemeField::Default);
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

    draw_end_of_buffer(&mut grid, view, theme);
    draw_trailing_whitespace(&mut grid, view, theme);
    if let Some(cmat) = win.search.cmatch.as_ref().cloned() {
        draw_search_highlights(&mut grid, &[cmat], view, theme);
    }
    draw_secondary_cursors(&mut grid, cursors, view, theme);
    let cursor = draw_primary_cursor(&mut grid, cursors.primary(), view, theme);
    redraw::Window::new(grid, cursor)
}

fn draw_search_highlights(
    grid: &mut Vec<Vec<redraw::Cell>>,
    matches: &[Range<usize>],
    view: &View,
    theme: &Theme,
) {
    let style = theme.get(ThemeField::Match);

    let vrange = view.range();
    for m in matches {
        if !vrange.overlaps(&m) {
            continue;
        }

        // TODO optimize
        let mut pos = vrange.start;
        for (line, row) in view.cells().iter().enumerate() {
            for (col, cell) in row.iter().enumerate() {
                if !matches!(cell, Cell::Empty) && m.contains(&pos) {
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
    for cursor in cursors.cursors() {
        if cursor == cursors.primary() {
            continue;
        }

        if !view.contains(cursor.pos()) {
            continue;
        }

        let _selection = cursor.selection();
        let (area, style) = match cursor.selection() {
            Some(s) => (s, theme.get(ThemeField::Selection)),
            None => (
                cursor.pos()..cursor.pos() + 1,
                theme.get(ThemeField::Cursor),
            ),
        };
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
    let style = theme.get(ThemeField::Selection);

    if let Some(area) = cursor.selection() {
        highlight_area(grid, area, view, style);
    }

    let has_selection = cursor.selection().is_some();
    let shape = if has_selection {
        CursorShape::Line(false)
    } else {
        CursorShape::Block(true)
    };
    let point = view
        .point_at_pos(cursor.pos())
        .expect("Primary cursor not in view");
    let style = theme.get(ThemeField::PrimaryCursor);
    redraw::Cursor {
        bg: style.bg,
        fg: style.fg,
        shape,
        point,
    }
}

fn draw_end_of_buffer(grid: &mut Vec<Vec<redraw::Cell>>, view: &View, theme: &Theme) {
    let style = theme.get(ThemeField::EndOfBuffer);
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

fn draw_trailing_whitespace(_grid: &mut Vec<Vec<redraw::Cell>>, view: &View, _theme: &Theme) {
    for (_line, _row) in view.cells().iter().enumerate() {}
}
