use std::ops::DerefMut as _;

use sanedit_messages::{
    redraw::{
        self, window::WindowGrid, Component, CursorShape, Point, Redraw, Style, Theme, ThemeField,
    },
    ClientMessage,
};
use sanedit_server::{FromEditor, FromEditorSharedMessage};
use sanedit_utils::sorted_vec::SortedVec;

use crate::editor::{
    buffers::Buffer,
    lsp::get_diagnostics,
    windows::{Cell, Cursors, Focus, Mode, View},
};

use super::{DrawContext, EditorContext};
use sanedit_core::{
    grapheme_category, BufferRange, Cursor, Diagnostic, GraphemeCategory, Range, Replacement,
};

pub(crate) fn draw(ctx: &mut DrawContext) -> FromEditorSharedMessage {
    let EditorContext {
        win,
        theme,
        buf,
        language_servers,
        ..
    } = ctx.editor;

    let mut window_buffer = ctx.state.window_buffer.blocking_lock();
    let window_grid =
        if let FromEditor::Message(ClientMessage::Redraw(Redraw::Window(Component::Open(win)))) =
            window_buffer.deref_mut()
        {
            win
        } else {
            unreachable!()
        };
    let grid = &mut window_grid.cells;
    if let Some(game) = &win.game {
        game.draw(grid, theme);
        window_grid.cursor = None;
        return FromEditorSharedMessage::Shared {
            message: ctx.state.window_buffer.clone(),
        };
    }

    let style = theme.get(ThemeField::Default);
    let vstyle = theme.get(ThemeField::Virtual);
    let view = win.view();
    if grid.height() != view.height() || grid.width() != view.width() {
        *grid = WindowGrid::new(view.width(), view.height(), redraw::Cell::with_style(style));
    }

    grid.clear_with(redraw::Cell::with_style(style));

    for (line, row) in view.cells().iter().enumerate() {
        for (col, cell) in row.iter().enumerate() {
            if cell.width() == 0 {
                continue;
            }

            // Has to be char because it has width
            let ch = cell.char().unwrap();
            grid.draw(
                line,
                col,
                redraw::Cell {
                    text: ch.display().into(),
                    style: if cell.is_virtual() { vstyle } else { style },
                },
            );

            // for i in 1..cell.width() {
            //     grid.draw(line, col + i, redraw::Cell::with_style(style));
            // }
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

    draw_syntax(grid, view, theme);
    if let Some(diagnostics) = diagnostics {
        draw_diagnostics(grid, diagnostics, view, theme);
    }
    draw_end_of_buffer(grid, view, theme);
    draw_trailing_whitespace(grid, view, theme, buf);
    if let Some(hls) = win.search.highlights() {
        draw_search_highlights(grid, &hls.highlights, view, theme);
    }
    draw_secondary_cursors(grid, cursors, focus_on_win, view, theme);
    window_grid.cursor = draw_primary_cursor(
        grid,
        cursors.primary(),
        can_insert && focus_on_win,
        view,
        theme,
    );

    FromEditorSharedMessage::Shared {
        message: ctx.state.window_buffer.clone(),
    }
}

fn draw_syntax(grid: &mut WindowGrid, view: &View, theme: &Theme) {
    const HL_PREFIX: &str = "window.view.";
    let syntax = view.syntax();
    draw_ordered_highlights(syntax.spans(), grid, view, |span| {
        if !span.highlight() {
            return None;
        }
        let style = theme.get(HL_PREFIX.to_owned() + span.name());
        Some((style, span.range()))
    });
}

fn draw_ordered_highlights<T, F>(items: &[T], grid: &mut WindowGrid, view: &View, f: F)
where
    F: Fn(&T) -> Option<(Style, &BufferRange)>,
{
    let vrange = view.range();
    // Should be advanced to range start at most
    let mut point = Point::default();
    let mut ppos = view.start();

    for item in items {
        let Some((style, range)) = (f)(item) else {
            continue;
        };

        if vrange.end < range.start {
            return;
        }

        if range.end < vrange.start {
            continue;
        }

        let mut start_found = false;
        let row_offset = point.y;
        let mut col_offset = point.x;
        let mut pos = ppos;

        'outer: for i in 0..(view.cells().len() - row_offset) {
            let line = row_offset + i;
            let row = &view.cells()[line];
            for j in 0..(row.len() - col_offset) {
                let col = col_offset + j;
                let cell = &view.cells()[line][col];

                if (!cell.is_virtual() && !cell.is_empty() && range.contains(&pos)) || cell.is_eof()
                {
                    grid.at(line, col).style = style;
                }

                if !start_found && pos + cell.len_in_buffer() >= range.start {
                    point.y = line;
                    point.x = col;
                    ppos = pos;
                    start_found = true;
                }

                pos += cell.len_in_buffer();

                if pos >= range.end {
                    break 'outer;
                }
            }

            col_offset = 0;
        }
    }
}

fn draw_diagnostics(grid: &mut WindowGrid, diagnostics: &[Diagnostic], view: &View, theme: &Theme) {
    const HL_PREFIX: &str = "window.view.";
    draw_ordered_highlights(diagnostics, grid, view, |diag| {
        let key = format!("{}{}", HL_PREFIX, diag.severity().as_ref());
        let style = theme.get(key);
        Some((style, diag.range()))
    });
}

// Prefer draw_ordered_highlights for multiple hls
fn draw_sigle_hl(grid: &mut WindowGrid, view: &View, style: Style, range: &BufferRange) {
    let vrange = view.range();
    if !vrange.overlaps(range) {
        return;
    }

    let mut pos = vrange.start;
    if let Some(point) = view.point_at_pos(pos) {
        for (i, row) in view.cells().iter().skip(point.y).enumerate() {
            let line = point.y + i;
            for (col, cell) in row.iter().enumerate() {
                if (!cell.is_virtual() && !cell.is_empty() && range.contains(&pos)) || cell.is_eof()
                {
                    grid.at(line, col).style = style;
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
    grid: &mut WindowGrid,
    matches: &SortedVec<BufferRange>,
    view: &View,
    theme: &Theme,
) {
    let style = theme.get(ThemeField::Match);
    draw_ordered_highlights(matches, grid, view, |hl| Some((style, hl)));
}

fn draw_secondary_cursors(
    grid: &mut WindowGrid,
    cursors: &Cursors,
    focus_on_win: bool,
    view: &View,
    theme: &Theme,
) {
    let mut cursor_hls = SortedVec::new();
    for cursor in cursors.cursors() {
        match cursor.selection() {
            Some(area) => {
                let style = theme.get(ThemeField::Selection);
                cursor_hls.push((area, style));
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

                cursor_hls.push((Range::new(cursor.pos(), cursor.pos() + 1), style));
            }
        }
    }

    draw_ordered_highlights(&cursor_hls, grid, view, |(range, style)| {
        Some((*style, range))
    });
}

fn draw_primary_cursor(
    grid: &mut WindowGrid,
    cursor: &Cursor,
    show_as_line: bool,
    view: &View,
    theme: &Theme,
) -> Option<redraw::Cursor> {
    let style = theme.get(ThemeField::Selection);

    if let Some(area) = cursor.selection() {
        draw_sigle_hl(grid, view, style, &area);
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

fn draw_end_of_buffer(grid: &mut WindowGrid, view: &View, theme: &Theme) {
    let style = theme.get(ThemeField::EndOfBuffer);
    for (line, row) in view.cells().iter().enumerate() {
        let is_empty = row.iter().all(|cell| matches!(cell, Cell::Empty));
        if is_empty {
            if let Some(rep) = view.options.replacements.get(&Replacement::BufferEnd) {
                grid.draw(line, 0, redraw::Cell::new_char(*rep, style));
            }
        }
    }
}

fn draw_trailing_whitespace(grid: &mut WindowGrid, view: &View, theme: &Theme, buf: &Buffer) {
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
                grid.draw(point.y, point.x, redraw::Cell::new_char(*rep, style));
            }
        }
    }
}
