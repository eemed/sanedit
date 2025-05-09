use std::cmp::min;

use sanedit_core::{word_at_pos, SearchOptions, Searcher};

use crate::{
    actions::jobs,
    editor::{
        windows::{Focus, HistoryKind, Prompt},
        Editor,
    },
};

use sanedit_server::ClientId;

use super::{window::focus, ActionResult};

const HORIZON_TOP: u64 = 1024 * 8;
const HORIZON_BOTTOM: u64 = 1024 * 16;

/// setups async job to handle matches within the view range.
fn async_view_matches(editor: &mut Editor, id: ClientId, pattern: &str) {
    let (win, buf) = editor.win_buf_mut(id);
    let pt = buf.ro_view();
    let mut view = win.view().range();
    view.start = view.start.saturating_sub(HORIZON_TOP);
    view.end = min(pt.len(), view.end + HORIZON_BOTTOM);
    let Ok((searcher, _)) = Searcher::new(pattern) else {
        return;
    };

    let job = jobs::Search::new(id, searcher, pt, view);
    editor.job_broker.request(job);
}

#[action("Search: Forward")]
fn search_forward(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    win.search.highlights.clear();

    win.prompt = Prompt::builder()
        .prompt("Search")
        .history(HistoryKind::Search)
        .on_confirm(|editor, id, out| {
            let needle = get!(out.text());
            new_search(editor, id, needle, false);
        })
        .on_input(async_view_matches)
        .build();
    focus(editor, id, Focus::Search);
    ActionResult::Ok
}

#[action("Search: Backward")]
fn search_backward(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);

    win.search.highlights.clear();

    win.prompt = Prompt::builder()
        .prompt("Search")
        .history(HistoryKind::Search)
        .on_confirm(|editor, id, out| {
            let needle = get!(out.text());
            new_search(editor, id, needle, true);
        })
        .on_input(async_view_matches)
        .build();

    focus(editor, id, Focus::Search);
    ActionResult::Ok
}

#[action("Search: Find word under cursor and move to next occurence")]
fn search_next_word_under_cursor(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, buf) = editor.win_buf_mut(id);
    let pos = win.cursors.primary().pos();
    let slice = buf.slice(..);
    let range = getf!(word_at_pos(&slice, pos));
    let word = String::from(&slice.slice(range));

    new_search(editor, id, &word, false);
    ActionResult::Ok
}

#[action("Search: Find word under cursor and move to previous occurence")]
fn search_prev_word_under_cursor(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, buf) = editor.win_buf_mut(id);
    let pos = win.cursors.primary().pos();
    let slice = buf.slice(..);
    let range = getf!(word_at_pos(&slice, pos));
    let word = String::from(&slice.slice(range));

    new_search(editor, id, &word, true);
    ActionResult::Ok
}

#[action("Editor: Clear match highlighting")]
fn clear_search_matches(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    win.search.highlights.clear();
    ActionResult::Ok
}

#[action("Search: Goto next match")]
fn next_search_match(editor: &mut Editor, id: ClientId) -> ActionResult {
    continue_search(editor, id, false);
    ActionResult::Ok
}

#[action("Search: Goto previous match")]
fn prev_search_match(editor: &mut Editor, id: ClientId) -> ActionResult {
    continue_search(editor, id, true);
    ActionResult::Ok
}

/// Continue current search
fn continue_search(editor: &mut Editor, id: ClientId, reverse: bool) {
    let (win, _buf) = editor.win_buf_mut(id);
    let mut pos = win.primary_cursor().pos();

    // If previous match move to the appropriate position
    if let Some(last_match) = &win.search.current.result {
        if last_match.contains(&pos) {
            if win.search.current.opts.is_reversed {
                pos = last_match.start;
            } else {
                pos = last_match.end;
            }
        }
    }

    let mut opts = win.search.current.opts;
    if reverse {
        opts.is_reversed = !opts.is_reversed;
    }

    let Ok(searcher) = Searcher::with_options(&win.search.current.pattern, &opts) else {
        return;
    };

    do_search(editor, id, searcher, pos)
}

/// Start a new search
fn new_search(editor: &mut Editor, id: ClientId, needle: &str, reverse: bool) {
    let (win, _buf) = editor.win_buf_mut(id);

    let (mut options, pattern) = SearchOptions::from_pattern(needle);
    options.is_reversed = reverse;
    let Ok(searcher) = Searcher::with_options(&pattern, &options) else {
        return;
    };
    win.search.current.pattern = pattern;
    win.search.current.opts = searcher.options();

    let cpos = win.cursors.primary().pos();
    do_search(editor, id, searcher, cpos)
}

fn do_search(editor: &mut Editor, id: ClientId, searcher: Searcher, starting_position: u64) {
    let (win, buf) = editor.win_buf_mut(id);

    let (start, mat, wrap) = if searcher.options().is_reversed {
        // Skip first to not match inplace
        let end = starting_position.saturating_sub(1);
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
        let start = min(blen, starting_position + 1);
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

            win.search.current.result = Some(range);

            if wrap {
                if searcher.options().is_reversed {
                    win.info_msg("Wrapped to end");
                } else {
                    win.info_msg("Wrapped to beginning");
                }
            }

            win.view_to_cursor(buf);
        }
        None => {
            win.search.current.result = None;
            win.warn_msg("No match found.");
        }
    }
}
