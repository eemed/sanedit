use sanedit_messages::redraw::{self, CursorShape, Style, Theme, ThemeField};

use crate::editor::{
    buffers::Buffer,
    lsp::get_diagnostics,
    windows::{Cell, Cursors, Focus, Mode, View},
};

use super::{DrawContext, EditorContext};
use sanedit_core::{
    grapheme_category, BufferRange, Cursor, Diagnostic, GraphemeCategory, Replacement,
};

pub(crate) fn draw(ctx: &mut DrawContext) -> redraw::window::Window {
    let EditorContext {
        win,
        theme,
        buf,
        language_servers,
        ..
    } = ctx.editor;

    let style = theme.get(ThemeField::Default);
    let vstyle = theme.get(ThemeField::Virtual);
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
                style: if cell.is_virtual() { vstyle } else { style },
            };

            for i in 1..cell.width() {
                grid[line][col + i] = redraw::Cell {
                    text: String::new(),
                    style,
                };
            }
        }
    }

    let focus_on_win = win.focus() == Focus::Window;
    let cursors = win.cursors();
    let diagnostics = if win.config.highlight_diagnostics {
        get_diagnostics(buf, language_servers)
    } else {
        None
    };
    // if layer does not discard, we can insert
    let can_insert = win.mode == Mode::Insert;

    draw_syntax(&mut grid, view, theme);
    if let Some(diagnostics) = diagnostics {
        draw_diagnostics(&mut grid, diagnostics, view, theme);
    }
    draw_end_of_buffer(&mut grid, view, theme);
    draw_trailing_whitespace(&mut grid, view, theme, buf);
    if let Some(hls) = win.search.highlights() {
        draw_search_highlights(&mut grid, &hls.highlights, view, theme);
    }
    draw_secondary_cursors(&mut grid, cursors, focus_on_win, view, theme);
    let cursor = draw_primary_cursor(
        &mut grid,
        cursors.primary(),
        can_insert && focus_on_win,
        view,
        theme,
    );

    redraw::window::Window {
        cells: grid,
        cursor,
    }
}

fn draw_syntax(grid: &mut Vec<Vec<redraw::Cell>>, view: &View, theme: &Theme) {
    const HL_PREFIX: &str = "window.view.";
    let syntax = view.syntax();

    for span in syntax.spans() {
        if !span.highlight() {
            continue;
        }
        let style = theme.get(HL_PREFIX.to_owned() + span.name());
        draw_hl(grid, view, style, &span.range());
    }
}

fn draw_diagnostics(
    grid: &mut [Vec<redraw::Cell>],
    diagnostics: &[Diagnostic],
    view: &View,
    theme: &Theme,
) {
    const HL_PREFIX: &str = "window.view.";

    for diag in diagnostics {
        let key = format!("{}{}", HL_PREFIX, diag.severity().as_ref());
        let style = theme.get(key);
        draw_hl(grid, view, style, &diag.range());
    }
}

fn draw_hl(grid: &mut [Vec<redraw::Cell>], view: &View, style: Style, range: &BufferRange) {
    let vrange = view.range();
    if vrange.start > range.end || range.start > vrange.end {
        return;
    }

    let mut pos = vrange.start;
    if let Some(point) = view.point_at_pos(pos) {
        for (i, row) in view.cells().iter().skip(point.y).enumerate() {
            let line = point.y + i;
            for (col, cell) in row.iter().enumerate() {
                if (!cell.is_virtual() && !cell.is_empty() && range.contains(&pos)) || cell.is_eof()
                {
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
    matches: &[BufferRange],
    view: &View,
    theme: &Theme,
) {
    let style = theme.get(ThemeField::Match);

    let vrange = view.range();
    for m in matches {
        if !vrange.overlaps(m) {
            continue;
        }

        draw_hl(grid, view, style, m);
    }
}

fn draw_secondary_cursors(
    grid: &mut Vec<Vec<redraw::Cell>>,
    cursors: &Cursors,
    focus_on_win: bool,
    view: &View,
    theme: &Theme,
) {
    for cursor in cursors.cursors() {
        match cursor.selection() {
            Some(area) => {
                let style = theme.get(ThemeField::Selection);
                draw_hl(grid, view, style, &area);
            }
            None => {
                let is_primary = cursor == cursors.primary();

                // Assume client draws the primary cursor here instead of us if
                // window is focused
                if is_primary && focus_on_win {
                    continue;
                }

                let style = if is_primary {
                    theme.get(ThemeField::Default)
                } else {
                    theme.get(ThemeField::Cursor)
                };

                if let Some(point) = view.point_at_pos(cursor.pos()) {
                    grid[point.y][point.x].style = style;
                }
            }
        }
    }
}

fn draw_primary_cursor(
    grid: &mut [Vec<redraw::Cell>],
    cursor: &Cursor,
    show_as_line: bool,
    view: &View,
    theme: &Theme,
) -> Option<redraw::Cursor> {
    let style = theme.get(ThemeField::Selection);

    if let Some(area) = cursor.selection() {
        draw_hl(grid, view, style, &area);
    }

    let line_style = cursor.selection().is_some() || show_as_line;
    let shape = if line_style {
        CursorShape::Line(false)
    } else {
        CursorShape::Block(true)
    };
    let point = view.point_at_pos(cursor.pos())?;
    let style = theme.get(ThemeField::Default);
    redraw::Cursor {
        bg: style.bg,
        fg: style.fg,
        shape,
        point,
    }
    .into()
}

fn draw_end_of_buffer(grid: &mut [Vec<redraw::Cell>], view: &View, theme: &Theme) {
    let style = theme.get(ThemeField::EndOfBuffer);
    for (line, row) in view.cells().iter().enumerate() {
        let is_empty = row.iter().all(|cell| matches!(cell, Cell::Empty));
        if is_empty {
            if let Some(rep) = view.options.replacements.get(&Replacement::BufferEnd) {
                grid[line][0] = redraw::Cell {
                    text: rep.to_string(),
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
            // Dont do tabs they are already done
            let is_tab = g == "\t";
            if is_tab {
                continue;
            }

            if let Some(point) = view.point_at_pos(g.start()) {
                grid[point.y][point.x] = redraw::Cell {
                    text: rep.to_string(),
                    style,
                };
            }
        }
    }
}
