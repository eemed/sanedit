use std::ops::Range;

use sanedit_messages::redraw::{self, CursorShape, Style, Theme, ThemeField};

use crate::{
    common::{
        char::{grapheme_category, GraphemeCategory, Replacement},
        range::RangeUtils,
    },
    editor::{
        buffers::{Buffer, BufferRange},
        windows::{Cell, Cursor, Cursors, View},
    },
};

use super::{DrawContext, EditorContext};

pub(crate) fn draw(ctx: &mut DrawContext) -> redraw::Window {
    let EditorContext {
        win, theme, buf, ..
    } = ctx.editor;

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

    draw_syntax(&mut grid, view, theme);
    draw_end_of_buffer(&mut grid, view, theme);
    draw_trailing_whitespace(&mut grid, view, theme, buf);
    draw_search_highlights(&mut grid, &win.search.hl_matches, view, theme);
    draw_secondary_cursors(&mut grid, cursors, view, theme);
    let cursor = draw_primary_cursor(&mut grid, cursors.primary(), view, theme);

    redraw::Window {
        cells: grid,
        cursor,
    }
}

fn draw_syntax(grid: &mut Vec<Vec<redraw::Cell>>, view: &View, theme: &Theme) {
    const HL_PREFIX: &str = "window.view.";
    let syntax = view.syntax();
    let vrange = view.range();

    for span in &syntax.highlights {
        if !span.highlight() {
            continue;
        }

        let srange = span.range();
        if !vrange.overlaps(&srange) {
            continue;
        }
        let style = theme.get(HL_PREFIX.to_owned() + span.name());

        draw_hl(grid, view, style, &srange);
    }
}

fn draw_hl(grid: &mut Vec<Vec<redraw::Cell>>, view: &View, style: Style, range: &BufferRange) {
    let vrange = view.range();
    if !vrange.overlaps(range) {
        return;
    }

    let mut pos = vrange.start;
    if let Some(point) = view.point_at_pos(pos) {
        for (i, row) in view.cells().iter().skip(point.y).enumerate() {
            let line = point.y + i;
            for (col, cell) in row.iter().enumerate() {
                if !matches!(cell, Cell::Empty) && range.contains(&pos) {
                    grid[line][col].style = style;
                }

                pos += cell.len_in_buffer();
            }

            if pos >= range.end {
                break;
            }
        }
    }
}

fn draw_search_highlights(
    grid: &mut Vec<Vec<redraw::Cell>>,
    matches: &[Range<u64>],
    view: &View,
    theme: &Theme,
) {
    let style = theme.get(ThemeField::Match);

    let vrange = view.range();
    for m in matches {
        if !vrange.overlaps(&m) {
            continue;
        }

        draw_hl(grid, view, style, m);
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
        draw_hl(grid, view, style, &area);
    }
}

fn draw_primary_cursor(
    grid: &mut Vec<Vec<redraw::Cell>>,
    cursor: &Cursor,
    view: &View,
    theme: &Theme,
) -> Option<redraw::Cursor> {
    let style = theme.get(ThemeField::Selection);

    if let Some(area) = cursor.selection() {
        draw_hl(grid, view, style, &area);
    }

    let has_selection = cursor.selection().is_some();
    let shape = if has_selection {
        CursorShape::Line(false)
    } else {
        CursorShape::Block(true)
    };
    let point = view.point_at_pos(cursor.pos())?;
    let style = theme.get(ThemeField::PrimaryCursor);
    redraw::Cursor {
        bg: style.bg,
        fg: style.fg,
        shape,
        point,
    }
    .into()
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

fn draw_trailing_whitespace(
    grid: &mut Vec<Vec<redraw::Cell>>,
    view: &View,
    theme: &Theme,
    buf: &Buffer,
) {
    let Some(rep) = view
        .options
        .replacements
        .get(&Replacement::TrailingWhitespace)
    else {
        return;
    };
    let style = theme.get(ThemeField::TrailingWhitespace);

    // Findout if last line includes only trailing whitespace
    let mut in_eol = true;
    let slice = buf.slice(view.range().end..);
    let mut graphemes = slice.graphemes_at(slice.len());
    while let Some(g) = graphemes.next() {
        let cat = grapheme_category(&g);
        match cat {
            GraphemeCategory::Whitespace => {}
            GraphemeCategory::EOL => {
                break;
            }
            _ => {
                in_eol = false;
                break;
            }
        }
    }

    // Iterate in reverse and mark all trailing whitespace
    let slice = buf.slice(view.range());
    let mut graphemes = slice.graphemes_at(slice.len());
    while let Some(g) = graphemes.prev() {
        let cat = grapheme_category(&g);
        match cat {
            GraphemeCategory::EOL => in_eol = true,
            GraphemeCategory::Whitespace => {}
            _ => in_eol = false,
        }

        if in_eol && cat == GraphemeCategory::Whitespace {
            if let Some(point) = view.point_at_pos(g.start()) {
                grid[point.y][point.x] = redraw::Cell {
                    text: rep.clone(),
                    style,
                };
            }
        }
    }
}

// pub(crate) fn draw_line_numbers(ctx: &DrawContext) -> LineNumbers {
//     let EditorContext { win, buf, .. } = ctx.editor;

//     let range = win.view().range();
//     let slice = buf.slice(range.clone());
//     let mut lines = slice.lines();
//     let mut line = lines.next();

//     let view = win.view();
//     let mut pos = view.start();
//     let mut lnr = buf.slice(..).line_at(view.start()) + 1;
//     let mut lnrs = vec![];

//     for row in view.cells() {
//         loop {
//             let on_next_line = line
//                 .as_ref()
//                 .map(|line| !line.is_empty() && line.end() <= pos)
//                 .unwrap_or(false);
//             if on_next_line {
//                 line = lines.next();
//                 if line.is_some() {
//                     lnr += 1;
//                 }
//             } else {
//                 break;
//             }
//         }

//         for cell in row {
//             pos += cell.len_in_buffer();
//         }

//         lnrs.push(lnr);
//     }

//     lnrs
// }
