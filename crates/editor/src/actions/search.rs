use std::cmp::min;

use sanedit_core::{word_at_pos, Range, SearchOptions, Searcher};

use crate::{
    actions::jobs,
    editor::{
        hooks::Hook,
        windows::{Focus, HistoryKind, Prompt, Window},
        Editor,
    },
};

use sanedit_server::ClientId;

use super::{window::focus, ActionResult};

const HORIZON_TOP: u64 = 1024;
const HORIZON_BOTTOM: u64 = 1024;

#[action("Adjust search highlights to take a buffer change into account")]
pub(crate) fn prevent_flicker(editor: &mut Editor, id: ClientId) -> ActionResult {
    fn add_offset(range: &mut Range<u64>, i: i128) {
        let neg = i.is_negative();
        let amount = i.unsigned_abs() as u64;
        if neg {
            range.start = range.start.saturating_sub(amount);
            range.end = range.end.saturating_sub(amount);
        } else {
            range.start += amount;
            range.end += amount;
        }
    }

    let (_win, buf) = editor.win_buf(id);
    let bid = buf.id;
    let bid = editor
        .hooks
        .running_hook()
        .and_then(Hook::buffer_id)
        .unwrap_or(bid);
    let clients = editor.windows().find_clients_with_buf(bid);

    for client in clients {
        let (win, buf) = editor.win_buf_mut(client);
        if let Some(old) = win.search.highlights_mut() {
            let edit = getf!(buf.last_edit());

            let mut off = 0i128;
            let mut iter = edit.changes.iter().peekable();

            // Highlights are just moved accordingly, order will not change
            unsafe {
                old.highlights.retain_mut(|hl| {
                    while let Some(next) = iter.peek() {
                        if next.end() <= hl.start {
                            // Before highlight
                            off -= next.range().len() as i128;
                            off += next.text().len() as i128;
                        } else if next.start() >= hl.end {
                            // Went past highlight
                            break;
                        } else if hl.includes(next.range()) {
                            // Inside a higlight assume the highlight spans this edit too
                            let removed = next.range().len() as i128;
                            let added = next.text().len() as i128;
                            off -= removed;
                            off += added;

                            // Extend or shrink instead
                            hl.end += added as u64;
                            hl.end = hl.end.saturating_sub(removed as u64);
                        } else {
                            // When edit is over highlight boundary just remove the higlight
                            return false;
                        }

                        iter.next();
                    }

                    add_offset(hl, off);
                    true
                });
            }
        }
    }

    ActionResult::Ok
}

/// setups async job to handle matches within the view range.
fn highlight_view_matches_on_input(editor: &mut Editor, id: ClientId, pattern: &str) {
    let Ok((searcher, _)) = Searcher::new(pattern) else {
        let (win, _buf) = editor.win_buf_mut(id);
        win.search.reset_highlighting();
        return;
    };
    highlight_view_matches(editor, id, searcher)
}

fn highlight_current_search_matches(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    let mut opts = win.search.current.opts;
    opts.is_reversed = false;

    let Ok(searcher) = Searcher::with_options(&win.search.current.pattern, &opts) else {
        return;
    };
    highlight_view_matches(editor, id, searcher)
}

/// Highlights search matches on view using
fn highlight_view_matches(editor: &mut Editor, id: ClientId, searcher: Searcher) {
    const JOB_NAME: &str = "search-highlight";
    let (win, buf) = editor.win_buf_mut(id);
    let pt = buf.ro_view();
    let mut view = win.view().range();
    view.start = view.start.saturating_sub(HORIZON_TOP);
    view.end = min(pt.len(), view.end + HORIZON_BOTTOM);
    let job = jobs::Search::new(id, searcher, buf.id, pt, view, buf.total_changes_made());
    editor.job_broker.request_slot(id, JOB_NAME, job);
}

#[action("Search: Highlight matches")]
fn highlight_search(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (_win, buf) = editor.win_buf_mut(id);
    let bid = buf.id;
    let bid = editor
        .hooks
        .running_hook()
        .and_then(Hook::buffer_id)
        .unwrap_or(bid);
    let clients = editor.windows().find_clients_with_buf(bid);

    for client in clients {
        let (win, buf) = editor.win_buf_mut(client);
        if !win.search.is_highlighting_enabled() {
            continue;
        }
        win.redraw_view(buf);
        let total = buf.total_changes_made();

        let view = win.view().range();
        let old = win.search.highlights().unwrap();

        if old.changes_made != total || !old.buffer_range.includes(view) {
            highlight_current_search_matches(editor, client);
        }
    }

    ActionResult::Ok
}

#[action("Search: Forward")]
fn search_forward(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    win.search.disable_highlighting();

    win.prompt = Prompt::builder()
        .prompt("Search")
        .history(HistoryKind::Search)
        .on_confirm(|editor, id, out| {
            let needle = getf!(out.text());
            new_search(editor, id, needle, false);
            ActionResult::Ok
        })
        .on_input(highlight_view_matches_on_input)
        .on_abort(|editor, id, _| {
            let (win, _buf) = editor.win_buf_mut(id);
            win.search.disable_highlighting();
        })
        .build();
    focus(editor, id, Focus::Search);
    ActionResult::Ok
}

#[action("Search: Backward")]
fn search_backward(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);

    win.search.disable_highlighting();
    win.prompt = Prompt::builder()
        .prompt("Search backwards")
        .history(HistoryKind::Search)
        .on_confirm(|editor, id, out| {
            let needle = getf!(out.text());
            new_search(editor, id, needle, true);
            ActionResult::Ok
        })
        .on_input(highlight_view_matches_on_input)
        .on_abort(|editor, id, _| {
            let (win, _buf) = editor.win_buf_mut(id);
            win.search.disable_highlighting();
        })
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
    win.search.disable_highlighting();
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
    let pos = win.primary_cursor().pos();

    let mut opts = win.search.current.opts;
    if reverse {
        opts.is_reversed = !opts.is_reversed;
    }

    let Ok(searcher) = Searcher::with_options(&win.search.current.pattern, &opts) else {
        return;
    };

    // Trigger search highlighting
    win.search.enable_highlighting();
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
    // Trigger search highlighting
    win.search.enable_highlighting();
    win.search.reset_highlighting();
    do_search(editor, id, searcher, cpos)
}

/// Skip over a match if starting position is in a highlight
fn skip_highlighted(win: &Window, starting_position: u64, reverse: bool) -> u64 {
    win.search
        .highlights()
        .as_ref()
        .and_then(|hls| {
            for hl in hls.highlights.iter() {
                if hl.contains(&starting_position) {
                    if reverse {
                        return Some(hl.start);
                    } else {
                        return Some(hl.end);
                    }
                }
            }

            None
        })
        .unwrap_or(starting_position + if !reverse { 1 } else { 0 })
}

fn do_search(editor: &mut Editor, id: ClientId, searcher: Searcher, starting_position: u64) {
    let (win, buf) = editor.win_buf_mut(id);

    let (start, mat, wrap) = if searcher.options().is_reversed {
        // Skip current match if needed
        let pos = skip_highlighted(win, starting_position, true);
        let end = pos;
        let slice = buf.slice(..end);
        let mat = if !slice.is_empty() {
            let mut iter = searcher.find_iter(&slice);
            iter.next()
        } else {
            None
        };

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
        // Skip current match if needed
        let pos = skip_highlighted(win, starting_position, false);
        let start = min(blen, pos);
        let slice = buf.slice(start..);
        let mat = if !slice.is_empty() {
            let mut iter = searcher.find_iter(&slice);
            iter.next()
        } else {
            None
        };

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
            if wrap {
                if searcher.options().is_reversed {
                    win.info_msg("Wrapped to end");
                } else {
                    win.info_msg("Wrapped to beginning");
                }
            }

            let mut range = mat.range();
            range.start += start;
            range.end += start;

            win.jump_to_offset(range.start, buf);
            win.search.current.result = Some(range);
        }
        None => {
            win.search.current.result = None;
            win.warn_msg("No match found.");
        }
    }
}
