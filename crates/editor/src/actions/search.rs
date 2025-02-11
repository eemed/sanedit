use std::cmp::min;

use sanedit_core::{word_at_pos, SearchKind, Searcher};

use crate::{
    actions::jobs,
    editor::{
        keymap::KeymapKind,
        windows::{Focus, HistoryKind, Prompt},
        Editor,
    },
};

use sanedit_server::ClientId;

const HORIZON_TOP: u64 = 1024 * 8;
const HORIZON_BOTTOM: u64 = 1024 * 16;

/// setups async job to handle matches within the view range.
fn async_view_matches(editor: &mut Editor, id: ClientId, pattern: &str) {
    let (win, buf) = editor.win_buf_mut(id);
    let pt = buf.ro_view();
    let mut view = win.view().range();
    view.start = view.start.saturating_sub(HORIZON_TOP);
    view.end = min(pt.len(), view.end + HORIZON_BOTTOM);
    let kind = win.search.kind;

    let job = jobs::Search::new(id, pattern, pt, view, kind);
    editor.job_broker.request(job);
}

#[action("Highlight last search")]
fn highlight_last_search(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    if !win.search.hl_last {
        return;
    }

    // TODO adjust if buffer changed, same as syntaxhl

    if let Some(last) = win.search.last_search() {
        let pt = buf.ro_view();
        let mut view = win.view().range();
        view.start = view.start.saturating_sub(HORIZON_TOP);
        view.end = min(pt.len(), view.end + HORIZON_BOTTOM);

        let job = jobs::Search::new(id, &last.pattern, pt, view, last.kind);
        editor.job_broker.request(job);
    }
}

#[action("Search forward")]
fn search_forward(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.search.hl_last = false;
    win.prompt = Prompt::builder()
        .prompt("Search")
        .history(HistoryKind::Search)
        .keymap(KeymapKind::Search)
        .on_confirm(|editor, id, out| {
            let text = get!(out.text());
            search(editor, id, text);
        })
        .on_input(async_view_matches)
        .build();
    win.focus_to(Focus::Search);
}

#[action("Search backwards")]
fn search_backward(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.search.hl_last = false;
    win.prompt = Prompt::builder()
        .history(HistoryKind::Search)
        .keymap(KeymapKind::Search)
        .prompt("Backward search")
        .on_confirm(|editor, id, out| {
            let text = get!(out.text());
            search(editor, id, text);
        })
        .on_input(async_view_matches)
        .build();
    win.focus_to(Focus::Search);
}

#[action("Search word under cursor and move to next occurence")]
fn search_next_word_under_cursor(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    let pos = win.cursors.primary().pos();
    let slice = buf.slice(..);
    let range = get!(word_at_pos(&slice, pos));
    let word = String::from(&slice.slice(range));
    win.search.kind = SearchKind::Default(false);

    search(editor, id, &word);
}

#[action("Search word under cursor and move to previous occurence")]
fn search_prev_word_under_cursor(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    let pos = win.cursors.primary().pos();
    let slice = buf.slice(..);
    let range = get!(word_at_pos(&slice, pos));
    let word = String::from(&slice.slice(range));
    win.search.kind = SearchKind::Default(true);

    search(editor, id, &word);
}

#[action("Clear match highlighting")]
fn clear_search_matches(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.search.current_match = None;
    win.search.hl_matches.clear();
    win.search.hl_last = false;
}

#[action("Goto next match")]
fn next_search_match(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    let Some(input) = win.search.last_search_pattern().map(String::from) else {
        return;
    };
    search(editor, id, &input);
}

#[action("Goto previous match")]
fn prev_search_match(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);

    let Some(input) = win.search.last_search_pattern().map(String::from) else {
        return;
    };

    if !win.search.kind.can_reverse() {
        win.warn_msg("Search method is unable to search to the other direction. TODO workaround")
    }

    // search to opposite direction
    win.search.kind.reverse();

    search(editor, id, &input);

    let (win, _buf) = editor.win_buf_mut(id);
    win.search.kind.reverse();
}

/// Execute a search from primary cursor position
pub(crate) fn search(editor: &mut Editor, id: ClientId, input: &str) {
    let (win, _buf) = editor.win_buf_mut(id);
    let cpos = win.cursors.primary().pos();
    win.search.save_last_search(input);

    search_impl(editor, id, input, cpos);
}

/// Execute the search for input at position pos
///
/// The potential match is reported to win.search.current_match
/// If no match is found current match is set to None
///
fn search_impl(editor: &mut Editor, id: ClientId, input: &str, mut pos: u64) {
    let (win, buf) = editor.win_buf_mut(id);
    if input.is_empty() {
        return;
    }

    // If previous match move to the appropriate position
    if let Some(last_match) = win
        .search
        .last_search()
        .and_then(|ls| ls.current_match.as_ref())
    {
        if last_match.contains(&pos) {
            if win.search.kind.is_reversed() {
                pos = last_match.start;
            } else {
                pos = last_match.end;
            }
        }
    }

    let Ok(searcher) = Searcher::new(input, win.search.kind) else {
        return;
    };

    let (start, mat, wrap) = if win.search.kind.is_reversed() {
        // Skip first to not match inplace
        let end = pos.saturating_sub(1);
        let slice = buf.slice(..end);
        let mut iter = searcher.find_iter(&slice);
        let mat = iter.next();

        // Wrap if no match
        if mat.is_none() {
            // let first = pos.saturating_sub(input.len() as u64 - 1);
            let slice = buf.slice(..);
            let mut iter = searcher.find_iter(&slice);
            let mat = iter.next();
            (slice.start(), mat, true)
        } else {
            (slice.start(), mat, false)
        }
    } else {
        let blen = buf.len();
        // Skip first to not match inplace
        let start = min(blen, pos + 1);
        let slice = buf.slice(start..);
        let mut iter = searcher.find_iter(&slice);
        let mat = iter.next();

        // Wrap if no match
        if mat.is_none() {
            // let last = min(blen, pos + input.len() as u64);
            let slice = buf.slice(..);
            let mut iter = searcher.find_iter(&slice);
            let mat = iter.next();
            (slice.start(), mat, true)
        } else {
            (slice.start(), mat, false)
        }
    };

    match mat {
        Some(mat) => {
            let mut range = mat.range();
            range.start += start;
            range.end += start;

            let cursor = win.primary_cursor_mut();
            cursor.goto(range.start);

            win.search.current_match = Some(range);
            win.search.hl_last = true;

            if wrap {
                if win.search.kind.is_reversed() {
                    win.info_msg("Wrapped to end");
                } else {
                    win.info_msg("Wrapped to beginning");
                }
            }

            win.view_to_cursor(buf);
        }
        None => {
            win.search.current_match = None;
            win.warn_msg("No match found.");
        }
    }
}
